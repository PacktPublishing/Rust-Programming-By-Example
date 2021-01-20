// Spec found at https://tools.ietf.org/html/rfc959

/*
 * FIXME: Filezilla says: Le serveur ne supporte pas les caractères non-ASCII.
 * FIXME: ftp cli says "WARNING! 71 bare linefeeds received in ASCII mode" when retrieving a file.
 */

 /*
 Includes codes of Chapter 8 & 9 as well
 */

#![feature(proc_macro, conservative_impl_trait, generators)]

extern crate bytes;
#[macro_use]
extern crate cfg_if;
extern crate futures_await as futures;
extern crate time;
extern crate tokio_core;
extern crate tokio_io;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;

mod cmd; // FIXME: rename this module.
mod codec;
mod error;
mod ftp;
mod config;

use std::env;
use std::ffi::OsString;
use std::fs::{File, Metadata, create_dir, read_dir, remove_dir_all};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Component, Path, PathBuf, StripPrefixError};
use std::result;

use futures::{Sink, Stream};
use futures::prelude::{async, await};
use futures::stream::{SplitSink, SplitStream};
use tokio_core::reactor::{Core, Handle};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_io::AsyncRead;
use tokio_io::codec::Framed;

use cmd::{Command, TransferType};
use codec::{BytesCodec, FtpCodec};
use config::{DEFAULT_PORT, Config};
use error::{Error, Result};
use ftp::{Answer, ResultCode};

const CONFIG_FILE: &'static str = "config.toml";
const MONTHS: [&'static str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                                    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

type DataReader = SplitStream<Framed<TcpStream, BytesCodec>>;
type DataWriter = SplitSink<Framed<TcpStream, BytesCodec>>;
type Writer = SplitSink<Framed<TcpStream, FtpCodec>>;

cfg_if! {
    if #[cfg(windows)] {
        fn get_file_info(meta: &Metadata) -> (time::Tm, u64) {
            use std::os::windows::prelude::*;
            (time::at(time::Timespec::new((meta.last_write_time() / 10_000_000) as i64, 0)),
                      meta.file_size())
        }
    } else {
        fn get_file_info(meta: &Metadata) -> (time::Tm, u64) {
            use std::os::unix::prelude::*;
            (time::at(time::Timespec::new(meta.mtime(), 0)), meta.size())
        }
    }
}

// If an error occurs when we try to get file's information, we just return and don't send its info.
fn add_file_info(path: PathBuf, out: &mut Vec<u8>) {
    let extra = if path.is_dir() { "/" } else { "" };
    let is_dir = if path.is_dir() { "d" } else { "-" };

    let meta = match ::std::fs::metadata(&path) {
        Ok(meta) => meta,
        _ => return,
    };
    let (time, file_size) = get_file_info(&meta);
    let path = match path.to_str() {
        Some(path) => match path.split("/").last() {
            Some(path) => path,
            _ => return,
        },
        _ => return,
    };
    // TODO: maybe improve how we get rights in here?
    let rights = if meta.permissions().readonly() {
        "r--r--r--"
    } else {
        "rw-rw-rw-"
    };
    let file_str = format!("{is_dir}{rights} {links} {owner} {group} {size} {month} {day} {hour}:{min} {path}{extra}\r\n",
                           is_dir=is_dir,
                           rights=rights,
                           links=1, // number of links
                           owner="anonymous", // owner name
                           group="anonymous", // group name
                           size=file_size,
                           month=MONTHS[time.tm_mon as usize],
                           day=time.tm_mday,
                           hour=time.tm_hour,
                           min=time.tm_min,
                           path=path,
                           extra=extra);
    out.extend(file_str.as_bytes());
    println!("==> {:?}", &file_str);
}

#[allow(dead_code)]
struct Client {
    cwd: PathBuf,
    data_port: Option<u16>,
    data_reader: Option<DataReader>,
    data_writer: Option<DataWriter>, // TODO: do we really need to split the data socket? => NOPE
    handle: Handle,
    name: Option<String>,
    server_root: PathBuf,
    transfer_type: TransferType,
    writer: Writer,
    is_admin: bool,
    config: Config,
    waiting_password: bool,
}

impl Client {
    fn new(handle: Handle, writer: Writer, server_root: PathBuf, config: Config) -> Client {
        Client {
            cwd: PathBuf::from("/"),
            data_port: None,
            data_reader: None,
            data_writer: None,
            handle,
            name: None,
            server_root,
            transfer_type: TransferType::Ascii,
            writer,
            is_admin: false,
            config,
            waiting_password: false,
        }
    }

    fn is_logged(&self) -> bool {
        self.name.is_some() && self.waiting_password == false
    }

    #[async]
    fn handle_cmd(mut self, cmd: Command) -> Result<Self> {
        println!("Received command: {:?}", cmd);
        if self.is_logged() {
            match cmd {
                Command::Cwd(directory) => return Ok(await!(self.cwd(directory))?),
                Command::List(path) => return Ok(await!(self.list(path))?),
                Command::Pasv => return Ok(await!(self.pasv())?),
                Command::Port(port) => {
                    self.data_port = Some(port);
                    return Ok(await!(self.send(Answer::new(ResultCode::Ok,
                                                        &format!("Data port is now {}", port))))?);
                }
                Command::Pwd => {
                    let msg = format!("{}", self.cwd.to_str().unwrap_or("")); // small trick
                    if !msg.is_empty() {
                        let message = format!("\"{}\" ", msg);
                        return Ok(await!(self.send(Answer::new(ResultCode::PATHNAMECreated,
                                                               &message)))?);
                    } else {
                        return Ok(await!(self.send(Answer::new(ResultCode::FileNotFound,
                                                               "No such file or directory")))?);
                    }
                }
                Command::Retr(file) => return Ok(await!(self.retr(file))?),
                Command::Stor(file) => return Ok(await!(self.stor(file))?),
                Command::CdUp => {
                    if let Some(path) = self.cwd.parent().map(Path::to_path_buf) {
                        self.cwd = path;
                        prefix_slash(&mut self.cwd);
                    }
                    return Ok(await!(self.send(Answer::new(ResultCode::Ok, "Done")))?);
                }
                Command::Mkd(path) => return Ok(await!(self.mkd(path))?),
                Command::Rmd(path) => return Ok(await!(self.rmd(path))?),
                _ => (),
            }
        } else if self.name.is_some() && self.waiting_password {
            if let Command::Pass(content) = cmd {
                let mut ok = false;
                if self.is_admin {
                    ok = content == self.config.admin.as_ref().unwrap().password;
                } else {
                    for user in &self.config.users {
                        if Some(&user.name) == self.name.as_ref() {
                            if user.password == content {
                                ok = true;
                                break;
                            }
                        }
                    }
                }
                if ok {
                    self.waiting_password = false;
                    let name = self.name.clone().unwrap_or(String::new());
                    self = await!(
                        self.send(Answer::new(ResultCode::UserLoggedIn,
                                              &format!("Welcome {}", name))))?;
                } else {
                    self = await!(self.send(Answer::new(ResultCode::NotLoggedIn,
                                                        "Invalid password")))?;
                }
                return Ok(self);
            }
        }
        match cmd {
            Command::Auth =>
                self = await!(self.send(Answer::new(ResultCode::CommandNotImplemented,
                                                    "Not implemented")))?,
            Command::Quit => self = await!(self.quit())?,
            Command::Syst => {
                self = await!(self.send(Answer::new(ResultCode::Ok, "I won't tell!")))?;
            }
            Command::Type(typ) => {
                self.transfer_type = typ;
                self = await!(self.send(Answer::new(ResultCode::Ok,
                                                    "Transfer type changed successfully")))?;
            }
            Command::User(content) => {
                if content.is_empty() {
                    self = await!(self.send(Answer::new(ResultCode::InvalidParameterOrArgument,
                                                        "Invalid username")))?;
                } else {
                    let mut name = None;
                    let mut pass_required = true;

                    self.is_admin = false;
                    if let Some(ref admin) = self.config.admin {
                        if admin.name == content {
                            name = Some(content.clone());
                            pass_required = admin.password.is_empty() == false;
                            self.is_admin = true;
                        }
                    }
                    if name.is_none() {
                        for user in &self.config.users {
                            if user.name == content {
                                name = Some(content.clone());
                                pass_required = user.password.is_empty() == false;
                                break;
                            }
                        }
                    }
                    if name.is_none() {
                        self = await!(self.send(Answer::new(ResultCode::NotLoggedIn,
                                                "Unknown user...")))?;
                    } else {
                        self.name = name.clone();
                        if pass_required {
                            self.waiting_password = true;
                            self = await!(
                                self.send(Answer::new(ResultCode::UserNameOkayNeedPassword,
                                          &format!("Login OK, password needed for {}",
                                                   name.unwrap()))))?;
                        } else {
                            self.waiting_password = false;
                            self = await!(self.send(Answer::new(ResultCode::UserLoggedIn,
                                                    &format!("Welcome {}!", content))))?;
                        }
                    }
                }
            }
            Command::NoOp => self = await!(self.send(Answer::new(ResultCode::Ok,
                                                                 "Doing nothing")))?,
            Command::Unknown(s) =>
                self = await!(self.send(Answer::new(ResultCode::UnknownCommand,
                                                    &format!("\"{}\": Not implemented", s))))?,
            _ => {
                // It means that the user tried to send a command while they weren't logged yet.
                self = await!(self.send(Answer::new(ResultCode::NotLoggedIn,
                                                    "Please log first")))?;
            }
        }
        Ok(self)
    }

    fn close_data_connection(&mut self) {
        self.data_reader = None;
        self.data_writer = None;
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

    fn strip_prefix(self, dir: PathBuf) -> (Self, result::Result<PathBuf, StripPrefixError>) {
        let res = dir.strip_prefix(&self.server_root).map(|p| p.to_path_buf());
        (self, res)
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
                prefix_slash(&mut self.cwd);
                self = await!(self.send(Answer::new(ResultCode::RequestedFileActionOkay,
                                                    &format!("Directory changed to \"{}\"",
                                                             directory.display()))))?;
                return Ok(self)
            }
        }
        self = await!(self.send(Answer::new(ResultCode::FileNotFound,
                                            "No such file or directory")))?;
        Ok(self)
    }

    #[async]
    fn list(mut self, path: Option<PathBuf>) -> Result<Self> {
        if self.data_writer.is_some() {
            let path = self.cwd.join(path.unwrap_or_default());
            let directory = PathBuf::from(&path);
            let (new_self, res) = self.complete_path(directory);
            self = new_self;
            if let Ok(path) = res {
                self = await!(self.send(Answer::new(ResultCode::DataConnectionAlreadyOpen,
                                                    "Starting to list directory...")))?;
                let mut out = vec![];
                if path.is_dir() {
                    if let Ok(dir) = read_dir(path) {
                        for entry in dir {
                            if let Ok(entry) = entry {
                                if self.is_admin ||
                                   entry.path() != self.server_root.join(CONFIG_FILE) {
                                    add_file_info(entry.path(), &mut out);
                                }
                            }
                        }
                    } else {
                        self = await!(self.send(Answer::new(ResultCode::InvalidParameterOrArgument,
                                                            "No such file or directory")))?;
                        return Ok(self);
                    }
                } else if self.is_admin || path != self.server_root.join(CONFIG_FILE) {
                    add_file_info(path, &mut out);
                }
                self = await!(self.send_data(out))?;
                println!("-> and done!");
            } else {
                self = await!(self.send(Answer::new(ResultCode::InvalidParameterOrArgument,
                                                    "No such file or directory")))?;
            }
        } else {
            self = await!(self.send(Answer::new(ResultCode::ConnectionClosed,
                                                "No opened data connection")))?;
        }
        if self.data_writer.is_some() {
            self.close_data_connection();
            self = await!(self.send(Answer::new(ResultCode::ClosingDataConnection,
                                                "Transfer done")))?;
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
            self = await!(self.send(Answer::new(ResultCode::DataConnectionAlreadyOpen,
                                                "Already listening...")))?;
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

    #[async]
    fn quit(mut self) -> Result<Self> {
        if self.data_writer.is_some() {
            unimplemented!();
        } else {
            self = await!(self.send(Answer::new(ResultCode::ServiceClosingControlConnection,
                                                "Closing connection...")))?;
            self.writer.close()?;
        }
        Ok(self)
    }

    #[async]
    fn retr(mut self, path: PathBuf) -> Result<Self> {
        // TODO: check if multiple data connection can be opened at the same time.
        if self.data_writer.is_some() {
            let path = self.cwd.join(path);
            let (new_self, res) = self.complete_path(path.clone()); // TODO: ugly clone
            self = new_self;
            if let Ok(path) = res {
                if path.is_file() && (self.is_admin || path != self.server_root.join(CONFIG_FILE)) {
                    self = await!(self.send(Answer::new(ResultCode::DataConnectionAlreadyOpen,
                                                        "Starting to send file...")))?;
                    let mut file = File::open(path)?;
                    let mut out = vec![];
                    // TODO: send the file chunck by chunck if it is big (if needed).
                    file.read_to_end(&mut out)?;
                    self = await!(self.send_data(out))?;
                    println!("-> file transfer done!");
                } else {
                    self = await!(self.send(Answer::new(ResultCode::LocalErrorInProcessing,
                                      &format!("\"{}\" doesn't exist", path.to_str()
                                               .ok_or_else(||
                                                   Error::Msg("No path".to_string()))?))))?;
                }
            } else {
                self = await!(self.send(Answer::new(ResultCode::LocalErrorInProcessing,
                                      &format!("\"{}\" doesn't exist", path.to_str()
                                               .ok_or_else(||
                                                   Error::Msg("No path".to_string()))?))))?;
            }
        } else {
            self = await!(self.send(Answer::new(ResultCode::ConnectionClosed,
                                                "No opened data connection")))?;
        }
        if self.data_writer.is_some() {
            self.close_data_connection();
            self = await!(self.send(Answer::new(ResultCode::ClosingDataConnection,
                                                "Transfer done")))?;
        }
        Ok(self)
    }

    #[async]
    fn stor(mut self, path: PathBuf) -> Result<Self> {
        if self.data_reader.is_some() {
            if invalid_path(&path) ||
               (!self.is_admin && path == self.server_root.join(CONFIG_FILE)) {
                let error: io::Error = io::ErrorKind::PermissionDenied.into();
                return Err(error.into());
            }
            let path = self.cwd.join(path);
            self = await!(self.send(Answer::new(ResultCode::DataConnectionAlreadyOpen,
                                                "Starting to send file...")))?;
            let (data, new_self) = await!(self.receive_data())?;
            self = new_self;
            let mut file = File::create(path)?;
            file.write_all(&data)?;
            println!("-> file transfer done!");
            self.close_data_connection();
            self = await!(self.send(Answer::new(ResultCode::ClosingDataConnection,
                                                "Transfer done")))?;
        } else {
            self = await!(self.send(Answer::new(ResultCode::ConnectionClosed,
                                                "No opened data connection")))?;
        }
        Ok(self)
    }

    #[async]
    fn receive_data(mut self) -> Result<(Vec<u8>, Self)> {
        let mut file_data = vec![];
        // NOTE: have to use this weird trick because of futures-await.
        // TODO: fix that when the lifetime stuff is improved for generators.
        if self.data_reader.is_none() {
            return Ok((vec![], self));
        }
        let reader = self.data_reader.take()
                                     .ok_or_else(|| Error::Msg("No data reader".to_string()))?;
        #[async]
        for data in reader {
            file_data.extend(&data);
        }
        Ok((file_data, self))
    }

    #[async]
    fn send(mut self, answer: Answer) -> Result<Self> {
        self.writer = await!(self.writer.send(answer))?;
        Ok(self)
    }

    #[async]
    fn send_data(mut self, data: Vec<u8>) -> Result<Self> {
        if let Some(writer) = self.data_writer {
            self.data_writer = Some(await!(writer.send(data))?);
        }
        Ok(self)
    }
}

#[async]
fn handle_client(stream: TcpStream, handle: Handle, server_root: PathBuf,
                 config: Config) -> result::Result<(), ()> {
    await!(client(stream, handle, server_root, config))
        .map_err(|error| println!("Error handling client: {}", error))
}

#[async]
fn client(stream: TcpStream, handle: Handle, server_root: PathBuf, config: Config) -> Result<()> {
    let (writer, reader) = stream.framed(FtpCodec).split();
    let writer = await!(writer.send(Answer::new(ResultCode::ServiceReadyForNewUser,
                                    "Welcome to this FTP server!")))?;
    let mut client = Client::new(handle, writer, server_root, config);
    #[async]
    for cmd in reader {
        client = await!(client.handle_cmd(cmd))?;
    }
    println!("Client closed");
    Ok(())
}

#[async]
fn server(handle: Handle, server_root: PathBuf, config: Config) -> io::Result<()> {
    let port = config.server_port.unwrap_or(DEFAULT_PORT);
    let addr = SocketAddr::new(IpAddr::V4(config.server_addr.as_ref()
                                                .unwrap_or(&"127.0.0.1".to_owned())
                                                .parse()
                                                .expect("Invalid IpV4 address...")),
                               port);
    let listener = TcpListener::bind(&addr, &handle)?;

    println!("Waiting clients on port {}...", port);
    #[async]
    for (stream, addr) in listener.incoming() {
        let address = format!("[address : {}]", addr);
        println!("New client: {}", address);
        handle.spawn(handle_client(stream, handle.clone(), server_root.clone(), config.clone()));
        println!("Waiting another client...");
    }
    Ok(())
}

fn invalid_path(path: &Path) -> bool {
    for component in path.components() {
        if let Component::ParentDir = component {
            return true;
        }
    }
    false
}

fn get_parent(path: PathBuf) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

fn get_filename(path: PathBuf) -> Option<OsString> {
    path.file_name().map(|p| p.to_os_string())
}

fn prefix_slash(path: &mut PathBuf) {
    if !path.is_absolute() {
        *path = Path::new("/").join(&path);
    }
}

fn main() {
    let config = Config::new(CONFIG_FILE).expect("Error while loading config...");
    let mut core = Core::new().expect("Cannot create tokio Core");
    let handle = core.handle();

    match env::current_dir() {
        Ok(server_root) => {
            if let Err(error) = core.run(server(handle, server_root, config)) {
                println!("Error running the server: {}", error);
            }
        }
        Err(e) => println!("Couldn't start server: {:?}", e),
    }
}
