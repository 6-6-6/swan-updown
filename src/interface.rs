use crate::misc;
use crate::netns;
use futures::TryStreamExt;
use log::{error, info, warn};
use netlink_packet_route::rtnl;
use netlink_packet_route::rtnl::link::nlas::Info;
use netlink_packet_route::rtnl::link::nlas::InfoData;
use netlink_packet_route::rtnl::link::nlas::InfoKind;
use netlink_packet_route::rtnl::link::nlas::InfoXfrmTun;
use netlink_packet_route::rtnl::link::nlas::Nla;
use std::os::unix::prelude::AsRawFd;

#[derive(Debug)]
pub enum GetResults {
    TypeNotMatch,
    IfIdNotMatch,
    NotFound,
    TokioJoinError,
    NoHandle,
}

// Add a new xfrm interface
pub async fn new_xfrm(interface: String, if_id: u32, alt_names: &[&str]) -> Result<(), ()> {
    info!(
        "adding new xfrm interface {}, if_id {}, altname {:#?}",
        interface, if_id, alt_names
    );

    let handle = misc::netlink_handle()?;
    let mut add_device_req = handle.link().add();
    let add_device_msg = add_device_req.message_mut();
    // header
    add_device_msg.header.link_layer_type = rtnl::ARPHRD_NONE;
    add_device_msg.header.flags = rtnl::IFF_UP | rtnl::IFF_MULTICAST | rtnl::IFF_NOARP;
    // set its name
    add_device_msg.nlas.push(Nla::IfName(interface.clone()));
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
        .map_err(|e| error!("Failed to add a XFRM interface {}: {}", interface, e))?;

    let mut add_prop_req = handle.link().property_add(0).alt_ifname(alt_names);
    let add_prop_msg = add_prop_req.message_mut();
    add_prop_msg.nlas.push(Nla::IfName(interface.clone()));
    #[allow(clippy::unit_arg)]
    add_prop_req.execute().await.map_or_else(
        |e| Ok(warn!("Failed to add altname for {}: {}", interface, e)),
        |_| Ok(()),
    )
}

// wrapper to delete an interface by its name
pub async fn del(interface: String) -> Result<(), ()> {
    info!("deleting interface {}", interface);

    let handle = misc::netlink_handle()?;
    let mut del_req = handle.link().del(0);

    let if_name = Nla::IfName(interface.clone());
    del_req.message_mut().nlas.push(if_name);

    del_req
        .execute()
        .await
        .map_err(|e| error!("Failed to del interface {}: {}", interface, e))
}

// move an interface to the given netns
pub async fn move_to_netns(interface: String, netns_name: &str) -> Result<(), ()> {
    info!("moving interface {} to netns {}", interface, netns_name);

    let handle = misc::netlink_handle()?;
    let netns_file = netns::get_netns_by_name(netns_name)?;

    if let Err(()) = handle
        .link()
        .set(0)
        .name(interface.clone())
        .setns_by_fd(netns_file.as_raw_fd())
        .up()
        .execute()
        .await
        .map_err(|e| error!("Failed to move {} to netns: {}", interface, e))
    {
        del(interface).await
    } else {
        Ok(())
    }
}

pub async fn get(name: String, expected_if_id: u32) -> Result<(), GetResults> {
    // log
    let handle = misc::netlink_handle().map_err(|_| GetResults::NoHandle)?;
    let mut links = handle.link().get().match_name(name.clone()).execute();
    //
    if let Some(link) = links.try_next().await.map_err(|_| GetResults::NotFound)? {
        let mut nlas = link.nlas.iter();
        while let Some(Nla::Info(infos)) = nlas.next() {
            for info in infos {
                match info {
                    Info::Kind(InfoKind::Xfrm) => continue,
                    Info::Kind(InfoKind::Other(desc)) => {
                        if desc.ne("xfrm") {
                            info!("get interface {}, but it is a {:?} device", name, desc);
                            return Err(GetResults::TypeNotMatch);
                        }
                    }
                    Info::Kind(kind) => {
                        info!("get interface {}, but it is a {:?} device", name, kind);
                        return Err(GetResults::TypeNotMatch);
                    }
                    Info::Data(InfoData::Xfrm(info_data)) => {
                        let mut info_data_iter = info_data.iter();
                        while let Some(InfoXfrmTun::IfId(if_id)) = info_data_iter.next() {
                            if expected_if_id.ne(if_id) {
                                info!("get interface {}, but if_id {} was not the expected value ({})", name, if_id, expected_if_id);
                                return Err(GetResults::IfIdNotMatch);
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
    info!("get interface {}, everything is ok", name);
    Ok(())
}

// wrapper to add an XFRM interface by its name in a given netns
pub async fn add_to_netns(
    netns_name: Option<String>,
    interface: String,
    if_id: u32,
    alt_names: &[&str],
) -> Result<(), ()> {
    match netns_name {
        None => new_xfrm(interface, if_id, alt_names).await,
        Some(my_netns_name) => {
            new_xfrm(interface.clone(), if_id, alt_names).await?;
            move_to_netns(interface, &my_netns_name).await
        }
    }
}

// wrapper to delete an interface by its name in a given netns
pub async fn del_in_netns(netns_name: Option<String>, interface: String) -> Result<(), ()> {
    match netns_name {
        None => del(interface).await,
        #[allow(clippy::unit_arg)]
        Some(my_netns_name) => netns::operate_in_netns(my_netns_name, del(interface))
            .await
            .unwrap_or_else(|e| Err(error!("{}", e))),
    }
}

// wrapper to get an interface by its name in a given netns
pub async fn get_in_netns(
    netns_name: Option<String>,
    interface: String,
    expected_if_id: u32,
) -> Result<(), GetResults> {
    match netns_name {
        None => get(interface, expected_if_id).await,
        Some(my_netns_name) => {
            netns::operate_in_netns(my_netns_name, get(interface, expected_if_id))
                .await
                .unwrap_or_else(|e| {
                    error!("{}", e);
                    Err(GetResults::TokioJoinError)
                })
        }
    }
}
