use log::{error, info};
use nix::sched::CloneFlags;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::unix::prelude::RawFd;
use std::path::Path;

// get netns File descriptor by its name
pub fn get_netns_by_name(name: &str) -> Result<File, ()> {
    info!("open netns {}", name);
    OpenOptions::new()
        .read(true)
        .open(Path::new("/run/netns").join(name))
        .map_err(|e| error!("Open netns {} failed: {}", name, e))
}

// after calling this function, the process will move into the given network namespace
pub fn into_netns_by_fd(netns_fd: RawFd, name: &str) -> Result<(), ()> {
    info!("switch to netns {}", name);
    let mut setns_flags = CloneFlags::empty();
    // unshare to the new network namespace
    nix::sched::unshare(CloneFlags::CLONE_NEWNET).map_err(|e| error!("unshare error: {e}"))?;
    // set netns
    setns_flags.insert(CloneFlags::CLONE_NEWNET);
    nix::sched::setns(netns_fd, setns_flags).map_err(|e| error!("setns error: {e}"))
}
