use log::{debug, error};
use std::path::Path;
use tokio::{io, net::UnixStream};

pub async fn add_interface(socket: &Path, interface: &str) -> Result<(), ()> {
    let command = format!("interface {}\nquit\n", interface).into_bytes();
    send_command(socket, &command).await
}

pub async fn del_interface(socket: &Path, interface: &str) -> Result<(), ()> {
    let command = format!("flush interface {}\nquit\n", interface).into_bytes();
    send_command(socket, &command).await
}

async fn send_command(socket: &Path, command: &[u8]) -> Result<(), ()> {
    let mut stream = UnixStream::connect(socket)
        .await
        .map_err(|e| error!("Failed to connect to babeld socket: {}", e))?;

    stream = read_from_stream(stream).await?;
    stream = write_to_stream(stream, command).await?;
    read_from_stream(stream).await?;

    Ok(())
}

async fn read_from_stream(stream: UnixStream) -> Result<UnixStream, ()> {
    let mut msg = vec![0; 1024];
    loop {
        // TODO: proper err msg
        stream
            .readable()
            .await
            .map_err(|e| error!("babeld socket error: {}", e))?;

        match stream.try_read(&mut msg) {
            Ok(n) => {
                msg.truncate(n);
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                error!("read error {}", e);
                return Err(());
            }
        }
    }
    debug!("{}", String::from_utf8_lossy(&msg));
    Ok(stream)
}

async fn write_to_stream(stream: UnixStream, command: &[u8]) -> Result<UnixStream, ()> {
    loop {
        // Wait for the socket to be writable
        stream
            .writable()
            .await
            .map_err(|e| error!("babeld socket error: {}", e))?;

        // Try to write data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_write(command) {
            Ok(_n) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                error!("write error {}", e);
                return Err(());
            }
        }
    }
    Ok(stream)
}
