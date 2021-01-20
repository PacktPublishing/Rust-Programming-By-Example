extern crate ftp;

use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use ftp::FtpStream;

struct ProcessController {
    child: Child,
}

impl ProcessController {
    fn new(child: Child) -> Self {
        ProcessController {
            child,
        }
    }

    fn is_running(&mut self) -> bool {
        let status = self.child.try_wait().unwrap();
        status.is_none()
    }
}

impl Drop for ProcessController {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn test_pwd() {
    let child =
        Command::new("./target/debug/ftp-server")
            .spawn().unwrap();
    let mut controller = ProcessController::new(child);

    thread::sleep(Duration::from_millis(100));
    assert!(controller.is_running(), "Server was aborted");

    let mut ftp = FtpStream::connect("127.0.0.1:1234").unwrap();

    let pwd = ftp.pwd().unwrap();
    assert_eq!("/", pwd);

    ftp.login("ferris", "").unwrap();

    ftp.cwd("src").unwrap();
    let pwd = ftp.pwd().unwrap();
    assert_eq!("/src", pwd);

    let _ = ftp.cdup();
    let pwd = ftp.pwd().unwrap();
    assert_eq!("/", pwd);

    ftp.quit().unwrap();
}
