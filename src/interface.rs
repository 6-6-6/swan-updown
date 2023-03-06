use crate::misc;
use crate::netns;
use log::{error, info};
use netlink_packet_route::rtnl;
use netlink_packet_route::rtnl::link::nlas::Info;
use netlink_packet_route::rtnl::link::nlas::InfoData;
use netlink_packet_route::rtnl::link::nlas::InfoKind;
use netlink_packet_route::rtnl::link::nlas::InfoXfrmTun;
use netlink_packet_route::rtnl::link::nlas::Nla;
use rtnetlink::Handle;
use std::os::unix::prelude::AsRawFd;

pub async fn new_xfrm(handle: &Handle, interface: &str, if_id: u32) -> Result<(), ()> {
    info!("add new xfrm interface {}, if_id {}", interface, if_id);

    let mut add_device_req = handle.link().add();
    let mut add_device_msg = add_device_req.message_mut();
    // header
    add_device_msg.header.link_layer_type = rtnl::ARPHRD_NONE;
    add_device_msg.header.flags = rtnl::IFF_UP | rtnl::IFF_MULTICAST;
    // set its name
    let if_name = Nla::IfName(interface.into());
    add_device_msg.nlas.push(if_name);
    // set the necessary info for adding a xfrm iface
    let xfrm_parent = InfoXfrmTun::Link(1);
    let xfrm_ifid = InfoXfrmTun::IfId(if_id);
    let xfrm_meta = Info::Data(InfoData::Xfrm(vec![xfrm_parent, xfrm_ifid]));
    let if_info = Nla::Info(vec![Info::Kind(InfoKind::Xfrm), xfrm_meta]);
    add_device_msg.nlas.push(if_info);
    // exec
    add_device_req
        .execute()
        .await
        .map_err(|e| error!("Failed to add a XFRM interface {}: {}", interface, e))
}

// wrapper to delete an interface by its name
pub async fn del(handle: &Handle, interface: &str) -> Result<(), ()> {
    info!("delete interface {}", interface);

    let mut del_req = handle.link().del(0);

    let if_name = Nla::IfName(interface.into());
    del_req.message_mut().nlas.push(if_name);

    del_req
        .execute()
        .await
        .map_err(|e| error!("Failed to del interface {}: {}", interface, e))
}

// wrapper to delete an interface by its name in a given netns
pub async fn del_in_netns(interface: &str, netns_name: &str) -> Result<(), ()> {
    let netns_file = netns::get_netns_by_name(netns_name)?;
    netns::into_netns_by_fd(netns_file.as_raw_fd(), netns_name)?;

    let handle = misc::netlink_handle()?;
    del(&handle, interface).await
}

// move an interface to the given netns
pub async fn move_to_netns(handle: &Handle, interface: &str, netns_name: &str) -> Result<(), ()> {
    info!("move interface {} to netns {}", interface, netns_name);

    let netns_file = netns::get_netns_by_name(netns_name)?;

    handle
        .link()
        .set(0)
        .name(interface.into())
        .setns_by_fd(netns_file.as_raw_fd())
        .up()
        .execute()
        .await
        .map_err(|e| error!("Failed to move {} to netns: {}", interface, e))
}
