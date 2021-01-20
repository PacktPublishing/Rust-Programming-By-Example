use std::path::{Path, PathBuf};
use std::str::{self, FromStr};

use error::{Error, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Auth,
    Cwd(PathBuf),
    List(Option<PathBuf>),
    Mkd(PathBuf),
    NoOp,
    Port(u16),
    Pass(String),
    Pasv,
    Pwd,
    Quit,
    Retr(PathBuf),
    Rmd(PathBuf),
    Stor(PathBuf),
    Syst,
    Type(TransferType),
    CdUp,
    Unknown(String),
    User(String),
}

impl AsRef<str> for Command {
    fn as_ref(&self) -> &str {
        match *self {
            Command::Auth => "AUTH",
            Command::Cwd(_) => "CWD",
            Command::List(_) => "LIST",
            Command::Pass(_) => "PASS",
            Command::Pasv => "PASV",
            Command::Port(_) => "PORT",
            Command::Pwd => "PWD",
            Command::Quit => "QUIT",
            Command::Retr(_) => "RETR",
            Command::Stor(_) => "STOR",
            Command::Syst => "SYST",
            Command::Type(_) => "TYPE",
            Command::User(_) => "USER",
            Command::CdUp => "CDUP",
            Command::Mkd(_) => "MKD",
            Command::Rmd(_) => "RMD",
            Command::NoOp => "NOOP",
            Command::Unknown(_) => "UNKN", // doesn't exist
        }
    }
}

impl Command {
    pub fn new(input: Vec<u8>) -> Result<Self> {
        let mut iter = input.split(|&byte| byte == b' ');
        let mut command = iter.next().ok_or_else(|| Error::Msg("empty command".to_string()))?.to_vec();
        to_uppercase(&mut command);
        let data = iter.next().ok_or_else(|| Error::Msg("no command parameter".to_string()));
        let command =
            match command.as_slice() {
                b"AUTH" => Command::Auth,
                b"CWD" => Command::Cwd(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf()))?),
                b"LIST" => Command::List(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf())).ok()),
                b"PASV" => Command::Pasv,
                b"PORT" => {
                    let addr = data?.split(|&byte| byte == b',')
                        .filter_map(|bytes| str::from_utf8(bytes).ok()
                                    .and_then(|string| u8::from_str(string).ok()))
                        .collect::<Vec<u8>>();
                    if addr.len() != 6 {
                        return Err("Invalid address/port".into());
                    }

                    let port = (addr[4] as u16) << 8 | (addr[5] as u16);
                    // TODO: check if the port isn't already used already by another connection...
                    if port <= 1024 {
                        return Err("Port can't be less than 10025".into());
                    }
                    Command::Port(port)
                }
                b"PWD" => Command::Pwd,
                b"QUIT" => Command::Quit,
                b"RETR" => Command::Retr(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf()))?),
                b"STOR" => Command::Stor(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf()))?),
                b"SYST" => Command::Syst,
                b"TYPE" => {
                    let error = Err("command not implemented for that parameter".into());
                    let data = data?;
                    if data.is_empty() {
                        return error;
                    }
                    match TransferType::from(data[0]) {
                        TransferType::Unknown => return error,
                        typ => {
                            Command::Type(typ)
                        },
                    }
                },
                b"CDUP" => Command::CdUp,
                b"MKD" => Command::Mkd(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf()))?),
                b"RMD" => Command::Rmd(data.and_then(|bytes| Ok(Path::new(str::from_utf8(bytes)?).to_path_buf()))?),
                b"USER" => Command::User(data.and_then(|bytes| String::from_utf8(bytes.to_vec()).map_err(Into::into))?),
                b"PASS" => Command::Pass(data.and_then(|bytes| String::from_utf8(bytes.to_vec()).map_err(Into::into))?),
                b"NOOP" => Command::NoOp,
                s => Command::Unknown(str::from_utf8(s).unwrap_or("").to_owned()),
            };
        Ok(command)
    }
}

fn to_uppercase(data: &mut [u8]) {
    for byte in data {
        if *byte >= 'a' as u8 && *byte <= 'z' as u8 {
            *byte -= 32;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TransferType {
    Ascii,
    Image,
    Unknown,
}

impl From<u8> for TransferType {
    fn from(c: u8) -> TransferType {
        match c {
            b'A' => TransferType::Ascii,
            b'I' => TransferType::Image,
            _ => TransferType::Unknown,
        }
    }
}
