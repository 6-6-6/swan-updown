use netlink_packet_route::rtnl;
use netlink_packet_route::rtnl::link::nlas::Info;
use netlink_packet_route::rtnl::link::nlas::InfoData;
use netlink_packet_route::rtnl::link::nlas::InfoKind;
use netlink_packet_route::rtnl::link::nlas::InfoXfrmTun;
use netlink_packet_route::rtnl::link::nlas::Nla;
use rtnetlink::{new_connection, Handle};
use std::fs::OpenOptions;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::path::Path;

use nix::sched::CloneFlags;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let (connection, handle, _) = new_connection().unwrap();
    let netns = OpenOptions::new()
        .read(true)
        .open(Path::new("/run/netns").join("test"))
        .unwrap();
    tokio::spawn(connection);

    println!("Hello, world!");
    new_device(&handle, "test-xfrm").await.unwrap();
    println!("Hello, world!");
    move_device_to_namespace(&handle, "test-xfrm", (&netns).as_raw_fd())
        .await
        .unwrap();
    println!("Hello, world!");
    del_device_netns("test-xfrm", (&netns).as_raw_fd())
        .await
        .unwrap();
    println!("Hello, world!");

    Ok(())
}

async fn new_device(handle: &Handle, interface: &str) -> Result<(), String> {
    let mut add_device_req = handle.link().add();
    let mut add_device_msg = add_device_req.message_mut();
    // header
    add_device_msg.header.link_layer_type = rtnl::ARPHRD_NONE;
    add_device_msg.header.flags = rtnl::IFF_UP | rtnl::IFF_MULTICAST;
    // body
    let if_name = Nla::IfName(interface.into());
    let if_parent = Nla::Link(1);
    let xfrm_parent = InfoXfrmTun::Link(1);
    let xfrm_ifid = InfoXfrmTun::IfId(114);
    let xfrm_meta = Info::Data(InfoData::Xfrm(vec![xfrm_parent, xfrm_ifid]));
    let if_info = Nla::Info(vec![Info::Kind(InfoKind::Xfrm), xfrm_meta]);
    add_device_msg.nlas.push(if_name);
    add_device_msg.nlas.push(if_parent);
    add_device_msg.nlas.push(if_info);
    // exec
    add_device_req.execute().await.map_err(|e| format!("{}", e))
}

async fn del_device(handle: &Handle, interface: &str) -> Result<(), String> {
    let mut del_req = handle.link().del(0);

    let if_name = Nla::IfName(interface.into());
    del_req.message_mut().nlas.push(if_name);
    //
    del_req.execute().await.map_err(|e| format!("{}", e))
}

// after calling this function, the process will move into the given network namespace
async fn del_device_netns(interface: &str, netns_fd: RawFd) -> Result<(), String> {
    let mut setns_flags = CloneFlags::empty();

    // unshare to the new network namespace
    if let Err(e) = nix::sched::unshare(CloneFlags::CLONE_NEWNET) {
        let err_msg = format!("unshare error: {e}");
        return Err(err_msg);
    }

    setns_flags.insert(CloneFlags::CLONE_NEWNET);
    if let Err(e) = nix::sched::setns(netns_fd, setns_flags) {
        let err_msg = format!("setns error: {e}");
        return Err(err_msg);
    };

    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);

    let mut del_req = handle.link().del(0);
    let if_name = Nla::IfName(interface.into());
    del_req.message_mut().nlas.push(if_name);
    del_req.execute().await.map_err(|e| format!("{}", e))
}

async fn move_device_to_namespace(
    handle: &Handle,
    interface: &str,
    namespace: RawFd,
) -> Result<(), String> {
    handle
        .link()
        .set(0)
        .name(interface.into())
        .setns_by_fd(namespace)
        .up()
        .execute()
        .await
        .map_err(|e| format!("{}", e))
}
