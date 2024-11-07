use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::AsFd;
use std::path::Path;
use std::thread;

use eyre::Error;
use futures::Future;
use log::info;
use nix::sched::CloneFlags;
use tokio::sync::oneshot::{self, error::RecvError};

// get netns File descriptor by its name
pub fn get_netns_by_name(name: &str) -> Result<File, std::io::Error> {
    info!("opening netns {}", name);
    OpenOptions::new()
        .read(true)
        .open(Path::new("/run/netns").join(name))
}

pub async fn operate_in_netns<T>(
    name: String,
    func: impl Future<Output = Result<(), T>> + Send + 'static,
) -> Result<Result<(), T>, RecvError>
where
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();

    thread::spawn(move || {
        // not likely to happen
        into_netns(&name).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let ret = rt.block_on(func);
        let _ = tx.send(ret);
    });

    rx.await
}

// after calling this function, the process will move into the given network namespace
fn into_netns(name: &str) -> Result<(), Error> {
    let netns_fd = get_netns_by_name(name)?;
    info!("switching to netns {}", name);
    let mut setns_flags = CloneFlags::empty();
    // unshare to the new network namespace
    nix::sched::unshare(CloneFlags::CLONE_NEWNET)?;
    // set netns
    setns_flags.insert(CloneFlags::CLONE_NEWNET);
    nix::sched::setns(netns_fd.as_fd(), setns_flags)?;
    Ok(())
}
