use eyre::{Error, WrapErr};
use log::trace;
use std::io;
use std::str;
use tokio::io::Interest;
use tokio::net::UnixStream;

pub async fn babeld_cmd(bind_path: &str, command: &str) -> Result<(), Error> {
    let stream = UnixStream::connect(bind_path)
        .await
        .wrap_err(format!("Could not connect to {}", bind_path))?;
    trace!("Connected to {}", bind_path);

    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;
        if ready.is_writable() {
            match stream.try_write(format!("{}\nquit\n", command).as_bytes()) {
                Ok(n) => {
                    trace!("Writing {} bytes to {}, cmd: {}", n, bind_path, command);
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    loop {
        let ready = stream
            .ready(Interest::READABLE | Interest::WRITABLE)
            .await?;
        if ready.is_readable() {
            let mut data = vec![0; 1500];
            match stream.try_read(&mut data) {
                Ok(n) => {
                    trace!("Reading {} bytes, msg: {}", n, str::from_utf8(&data[0..n])?);
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
