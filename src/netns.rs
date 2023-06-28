use futures::Future;
use log::{error, info};
use nix::sched::CloneFlags;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use tokio::task::JoinError;

// get netns File descriptor by its name
pub fn get_netns_by_name(name: &str) -> Result<File, ()> {
    info!("opening netns {}", name);
    OpenOptions::new()
        .read(true)
        .open(Path::new("/run/netns").join(name))
        .map_err(|e| error!("Open netns {} failed: {}", name, e))
}

pub async fn operate_in_netns<T>(
    name: String,
    func: impl Future<Output = Result<(), T>> + Send + 'static,
) -> Result<Result<(), T>, JoinError>
where
    T: Send + 'static,
{
    tokio::spawn(async move {
        // not likely to happen
        into_netns(&name).unwrap();
        func.await
    })
    .await
}

// after calling this function, the process will move into the given network namespace
fn into_netns(name: &str) -> Result<(), ()> {
    let netns_fd = get_netns_by_name(name)?;
    info!("switching to netns {}", name);
    let mut setns_flags = CloneFlags::empty();
    // unshare to the new network namespace
    nix::sched::unshare(CloneFlags::CLONE_NEWNET).map_err(|e| error!("Unshare error: {}", e))?;
    // set netns
    setns_flags.insert(CloneFlags::CLONE_NEWNET);
    nix::sched::setns(netns_fd.as_raw_fd(), setns_flags).map_err(|e| error!("Setns error: {}", e))
}
