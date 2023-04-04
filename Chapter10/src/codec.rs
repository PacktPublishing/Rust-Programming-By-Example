use std::io::{self, Write};

use bytes::BytesMut;
use tokio_io::codec::{Decoder, Encoder};

use cmd::Command;
use error::Error;
use ftp::Answer;

pub struct BytesCodec;
pub struct FtpCodec;

impl Decoder for FtpCodec {
    type Item = Command;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Command>> {
        if let Some(index) = find_crlf(buf) {
            let line = buf.split_to(index);
            buf.split_to(2); // Remove \r\n.
            Command::new(line.to_vec())
                .map(|command| Some(command))
                .map_err(Error::to_io_error)
        } else {
            Ok(None)
        }
    }
}

impl Encoder for FtpCodec {
    type Item = Answer;
    type Error = io::Error;

    fn encode(&mut self, answer: Answer, buf: &mut BytesMut) -> io::Result<()> {
        let mut buffer = vec![];
        if answer.message.is_empty() {
            write!(buffer, "{}\r\n", answer.code as u32)?;
        } else {
            write!(buffer, "{} {}\r\n", answer.code as u32, answer.message)?;
        }
        buf.extend(&buffer);
        Ok(())
    }
}

impl Decoder for BytesCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
        if buf.len() == 0 {
            return Ok(None);
        }
        let data = buf.to_vec();
        buf.clear();
        Ok(Some(data))
    }
}

impl Encoder for BytesCodec {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
        buf.extend(data);
        Ok(())
    }
}

fn find_crlf(buf: &mut BytesMut) -> Option<usize> {
    buf.windows(2)
        .position(|bytes| bytes == b"\r\n")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ftp::ResultCode;
    use super::{Answer, BytesMut, Command, Decoder, Encoder, FtpCodec};

    #[test]
    fn test_encoder() {
        let mut codec = FtpCodec;
        let message = "bad sequence of commands";
        let answer = Answer::new(ResultCode::BadSequenceOfCommands, message);
        let mut buf = BytesMut::new();
        let result = codec.encode(answer, &mut buf);
        assert!(result.is_ok());
        assert_eq!(buf, format!("503 {}\r\n", message));

        let answer = Answer::new(ResultCode::CantOpenDataConnection, "");
        let mut buf = BytesMut::new();
        let result = codec.encode(answer, &mut buf);
        assert!(result.is_ok(), "Result is ok");
        assert_eq!(buf, format!("425\r\n"), "Buffer contains 425");
    }

    #[test]
    fn test_decoder() {
        let mut codec = FtpCodec;
        let mut buf = BytesMut::new();
        buf.extend(b"PWD");
        let result = codec.decode(&mut buf);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert!(command.is_none());

        buf.extend(b"\r\n");
        let result = codec.decode(&mut buf);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert_eq!(command, Some(Command::Pwd));

        let mut buf = BytesMut::new();
        buf.extend(b"LIST /tmp\r\n");
        let result = codec.decode(&mut buf);
        assert!(result.is_ok());
        let command = result.unwrap();
        assert_eq!(command, Some(Command::List(Some(PathBuf::from("/tmp")))));
    }
}
