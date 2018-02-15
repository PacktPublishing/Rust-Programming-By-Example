#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate bytes;
extern crate futures_await as futures;
extern crate tokio_core;
extern crate tokio_io;

mod cmd;
mod codec;
mod error;
mod ftp;

use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::env;
use std::path::{PathBuf, StripPrefixError};
use std::result;

use futures::{Sink, Stream};
use futures::prelude::{async, await};
use futures::stream::SplitSink;
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;

use cmd::Command;
use cmd::TransferType;
use codec::FtpCodec;
use error::Result;
use ftp::{Answer, ResultCode};
// +++
use std::ffi::OsString;
use std::fs::{create_dir, remove_dir_all};
use std::path::Path;

use futures::stream::SplitStream;

use codec::BytesCodec;

type DataReader = SplitStream<Framed<TcpStream, BytesCodec>>;
type DataWriter = SplitSink<Framed<TcpStream, BytesCodec>>;
// +++

type Writer = SplitSink<Framed<TcpStream, FtpCodec>>;

#[allow(dead_code)]
struct Client {
    cwd: PathBuf,
    // +++
    data_port: Option<u16>,
    data_reader: Option<DataReader>,
    data_writer: Option<DataWriter>,
    handle: Handle,
    // +++
    server_root: PathBuf,
    transfer_type: TransferType,
    writer: Writer,
}

impl Client {
    // +++
    fn new(handle: Handle, writer: Writer, server_root: PathBuf) -> Client {
    // +++
        Client {
            cwd: PathBuf::from("/"),
            // +++
            data_port: None,
            data_reader: None,
            data_writer: None,
            handle,
            // +++
            server_root,
            transfer_type: TransferType::Ascii,
            writer,
        }
    }

    #[async]
    fn handle_cmd(mut self, cmd: Command) -> Result<Self> {
        println!("Received command: {:?}", cmd);
        match cmd {
            Command::Cwd(directory) => self = await!(self.cwd(directory))?,
            Command::Pwd => {
                let msg = format!("{}", self.cwd.to_str().unwrap_or("")); // small trick
                if !msg.is_empty() {
                    let message = format!("\"/{}\" ", msg);
                    self = await!(self.send(Answer::new(ResultCode::PATHNAMECreated, &message)))?;
                } else {
                    self = await!(self.send(Answer::new(ResultCode::FileNotFound, "No such file or directory")))?;
                }
            }
            // +++
            Command::Pasv => self = await!(self.pasv())?,
            Command::Port(port) => {
                self.data_port = Some(port);
                self = await!(self.send(Answer::new(ResultCode::Ok, &format!("Data port is now {}", port))))?;
            }
            Command::Quit => self = await!(self.quit())?,
            Command::Syst => {
                self = await!(self.send(Answer::new(ResultCode::Ok, "I won't tell!")))?;
            }
            Command::CdUp => {
                if let Some(path) = self.cwd.parent().map(Path::to_path_buf) {
                    self.cwd = path;
                }
                self = await!(self.send(Answer::new(ResultCode::Ok, "Done")))?;
            }
            Command::Mkd(path) => self = await!(self.mkd(path))?,
            Command::Rmd(path) => self = await!(self.rmd(path))?,
            Command::NoOp => self = await!(self.send(Answer::new(ResultCode::Ok, "Doing nothing")))?,
            // +++
            Command::Type(typ) => {
                self.transfer_type = typ;
                self = await!(self.send(Answer::new(ResultCode::Ok, "Transfer type changed successfully")))?;
            }
            Command::User(content) => {
                if content.is_empty() {
                    self = await!(self.send(Answer::new(ResultCode::InvalidParameterOrArgument, "Invalid username")))?;
                } else {
                    self = await!(self.send(Answer::new(ResultCode::UserloggedIn, &format!("Welcome {}!", content))))?;
                }
            }
            Command::Unknown(s) =>
                self = await!(self.send(Answer::new(ResultCode::UnknownCommand,
                                                    &format!("\"{}\": Not implemented", s))))?
            ,
            _ => self = await!(self.send(Answer::new(ResultCode::CommandNotImplemented, "Not implemented")))?,
        }
        Ok(self)
    }

    fn complete_path(self, path: PathBuf) -> (Self, result::Result<PathBuf, io::Error>) {
        let directory = self.server_root.join(if path.has_root() {
            path.iter().skip(1).collect()
        } else {
            path
        });
        let dir = directory.canonicalize();
        if let Ok(ref dir) = dir {
            if !dir.starts_with(&self.server_root) {
                return (self, Err(io::ErrorKind::PermissionDenied.into()));
            }
        }
        (self, dir)
    }

    #[async]
    fn cwd(mut self, directory: PathBuf) -> Result<Self> {
        let path = self.cwd.join(&directory);
        let (new_self, res) = self.complete_path(path);
        self = new_self;
        if let Ok(dir) = res {
            let (new_self, res) = self.strip_prefix(dir);
            self = new_self;
            if let Ok(prefix) = res {
                self.cwd = prefix.to_path_buf();
                self = await!(self.send(Answer::new(ResultCode::Ok,
                                                    &format!("Directory changed to \"{}\"", directory.display()))))?;
                return Ok(self)
            }
        }
        self = await!(self.send(Answer::new(ResultCode::FileNotFound,
                                            "No such file or directory")))?;
        Ok(self)
    }

    // +++
    #[async]
    fn mkd(mut self, path: PathBuf) -> Result<Self> {
        let path = self.cwd.join(&path);
        let parent = get_parent(path.clone());
        if let Some(parent) = parent {
            let parent = parent.to_path_buf();
            let (new_self, res) = self.complete_path(parent);
            self = new_self;
            if let Ok(mut dir) = res {
                if dir.is_dir() {
                    let filename = get_filename(path);
                    if let Some(filename) = filename {
                        dir.push(filename);
                        if create_dir(dir).is_ok() {
                            self = await!(self.send(Answer::new(ResultCode::PATHNAMECreated,
                                                                "Folder successfully created!")))?;
                            return Ok(self);
                        }
                    }
                }
            }
        }
        self = await!(self.send(Answer::new(ResultCode::FileNotFound,
                                            "Couldn't create folder")))?;
        Ok(self)
    }

    #[async]
    fn rmd(mut self, directory: PathBuf) -> Result<Self> {
        let path = self.cwd.join(&directory);
        let (new_self, res) = self.complete_path(path);
        self = new_self;
        if let Ok(dir) = res {
            if remove_dir_all(dir).is_ok() {
                self = await!(self.send(Answer::new(ResultCode::RequestedFileActionOkay,
                                                    "Folder successfully removed")))?;
                return Ok(self);
            }
        }
        self = await!(self.send(Answer::new(ResultCode::FileNotFound,
                                            "Couldn't remove folder")))?;
        Ok(self)
    }

    #[async]
    fn quit(mut self) -> Result<Self> {
        if self.data_writer.is_some() {
            unimplemented!();
        } else {
            self = await!(self.send(Answer::new(ResultCode::ServiceClosingControlConnection, "Closing connection...")))?;
            self.writer.close()?;
        }
        Ok(self)
    }

    #[async]
    fn pasv(mut self) -> Result<Self> {
        let port =
            if let Some(port) = self.data_port {
                port
            } else {
                0
            };
        if self.data_writer.is_some() {
            self = await!(self.send(Answer::new(ResultCode::DataConnectionAlreadyOpen, "Already listening...")))?;
            return Ok(self);
        }

        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
        let listener = TcpListener::bind(&addr, &self.handle)?;
        let port = listener.local_addr()?.port();

        self = await!(self.send(Answer::new(ResultCode::EnteringPassiveMode,
                              &format!("127,0,0,1,{},{}", port >> 8, port & 0xFF))))?;

        println!("Waiting clients on port {}...", port);
        // TODO: use into_future() instead of for loop?
        #[async]
        for (stream, _rest) in listener.incoming() {
            let (writer, reader) = stream.framed(BytesCodec).split();
            self.data_writer = Some(writer);
            self.data_reader = Some(reader);
            break;
        }
        Ok(self)
    }
    // +++

    fn strip_prefix(self, dir: PathBuf) -> (Self, result::Result<PathBuf, StripPrefixError>) {
        let res = dir.strip_prefix(&self.server_root).map(|p| p.to_path_buf());
        (self, res)
    }

    #[async]
    fn send(mut self, answer: Answer) -> Result<Self> {
        self.writer = await!(self.writer.send(answer))?;
        Ok(self)
    }
}

#[async]
// +++
fn handle_client(stream: TcpStream, handle: Handle, server_root: PathBuf) -> result::Result<(), ()> {
// +++
    await!(client(stream, handle, server_root))
        .map_err(|error| println!("Error handling client: {}", error))
}

#[async]
// +++
fn client(stream: TcpStream, handle: Handle, server_root: PathBuf) -> Result<()> {
// +++
    let (writer, reader) = stream.framed(FtpCodec).split();
    let writer = await!(writer.send(Answer::new(ResultCode::ServiceReadyForNewUser, "Welcome to this FTP server!")))?;
    // +++
    let mut client = Client::new(handle, writer, server_root);
    // +++
    #[async]
    for cmd in reader {
        client = await!(client.handle_cmd(cmd))?;
    }
    println!("Client closed");
    Ok(())
}

#[async]
fn server(handle: Handle, server_root: PathBuf) -> io::Result<()> {
    let port = 1234;
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
    let listener = TcpListener::bind(&addr, &handle)?;

    println!("Waiting clients on port {}...", port);
    #[async]
    for (stream, addr) in listener.incoming() {
        let address = format!("[address : {}]", addr);
        println!("New client: {}", address);
        // +++
        handle.spawn(handle_client(stream, handle.clone(), server_root.clone()));
        // +++
        println!("Waiting another client...");
    }
    Ok(())
}

fn get_parent(path: PathBuf) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

fn get_filename(path: PathBuf) -> Option<OsString> {
    path.file_name().map(|p| p.to_os_string())
}

fn main() {
    let mut core = Core::new().expect("Cannot create tokio Core");
    let handle = core.handle();

    match env::current_dir() {
        Ok(server_root) => {
            if let Err(error) = core.run(server(handle, server_root)) {
                println!("Error running the server: {}", error);
            }
        }
        Err(e) => println!("Couldn't start server: {:?}", e),
    }
}
