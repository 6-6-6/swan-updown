use std::os::unix::prelude::AsRawFd;

use futures::TryStreamExt;
use log::{debug, error, info, warn};
// for creating a link
use netlink_packet_route::link::LinkAttribute;
use netlink_packet_route::link::LinkFlag;
use netlink_packet_route::link::LinkLayerType;
// for get a link info
use netlink_packet_route::link::InfoData;
use netlink_packet_route::link::InfoKind;
use netlink_packet_route::link::InfoXfrm;
use netlink_packet_route::link::LinkInfo;
use netlink_packet_route::link::State;
//use netlink_packet_route::link::Nla;
use eyre::{Error, WrapErr};

use crate::misc;
use crate::netns;

#[derive(Debug)]
pub enum GetResults {
    TypeNotMatch,
    IfIdNotMatch,
    IfOperStateNotMatch,
    NotFound,
    TokioJoinError,
    NoHandle,
}

// Add a new xfrm interface
async fn new_xfrm(
    interface: String,
    if_id: u32,
    alt_names: &[&str],
    master_dev: Option<String>,
) -> Result<(), Error> {
    info!(
        "adding new xfrm interface {}, if_id {}, altname {:#?}",
        interface, if_id, alt_names
    );

    let mut handle = misc::netlink_handle()?;
    let mut add_device_req = handle.link().add().xfrmtun(interface.clone(), if_id);
    let add_device_msg = add_device_req.message_mut();
    // headers
    add_device_msg.header.link_layer_type = LinkLayerType::None; //rtnl::ARPHRD_NONE;
    add_device_msg.header.flags.push(LinkFlag::Multicast);
    add_device_msg.header.flags.push(LinkFlag::Noarp);
    add_device_msg.header.change_mask.push(LinkFlag::Multicast);
    add_device_msg.header.change_mask.push(LinkFlag::Noarp);
    // set the necessary info for adding a xfrm iface
    add_device_req
        .execute()
        .await
        .wrap_err_with(|| format!("Failed to add a XFRM interface {}", interface))?;

    // add the altname of the interface
    let mut add_prop_req = handle.link().property_add(0).alt_ifname(alt_names);
    let add_prop_msg = add_prop_req.message_mut();
    add_prop_msg
        .attributes
        .push(LinkAttribute::IfName(interface.clone()));
    #[allow(clippy::unit_arg)]
    // even if it fails, everything will be okay, so no error here
    if let Err(e) = add_prop_req.execute().await {
        warn!("Failed to add altname for {}: {}", interface, e)
    }
    // get the idx of the master device
    let master_devidx = misc::get_index_by_name(&mut handle, master_dev)
        .await?;
    // bring the interface up after creation
    if let Err(e) = handle
        .link()
        .set(0)
        .name(interface.clone())
        .up()
        .controller(master_devidx)
        .execute()
        .await
    {
      error!(
        "Failed to bring interface {} up: {}, deleting it...",
        interface, e
      );
      del(interface).await?;
    }

    Ok(())
}

// wrapper to delete an interface by its name
async fn del(interface: String) -> Result<(), Error> {
    info!("deleting interface {}", interface);

    let handle = misc::netlink_handle()?;
    let mut del_req = handle.link().del(0);

    del_req
        .message_mut()
        .attributes
        .push(LinkAttribute::IfName(interface.clone()));

    del_req
        .execute()
        .await
        .wrap_err_with(|| format!("Failed to del interface {}", interface))
}

// move an interface to the given netns
async fn move_to_netns(interface: &str, netns_name: &str) -> Result<(), Error> {
    info!("moving interface {} to netns {}", interface, netns_name);

    let handle = misc::netlink_handle()?;
    let netns_file = netns::get_netns_by_name(netns_name)?;

    handle
        .link()
        .set(0)
        .name(interface.to_owned())
        .setns_by_fd(netns_file.as_raw_fd())
        .up()
        .execute()
        .await?;

    Ok(())
}

async fn get(name: String, expected_if_id: u32) -> Result<(), GetResults> {
    // log
    let handle = misc::netlink_handle().map_err(|_| GetResults::NoHandle)?;
    let mut links = handle.link().get().match_name(name.clone()).execute();
    //
    if let Some(link) = links.try_next().await.map_err(|_| GetResults::NotFound)? {
        //let mut attrs = link.attributes.iter();
        // first of all, I need to check whether the interface is a proper xfrm interfaceå
        for link_attr in link.attributes.iter() {
            let mut is_xfrm = false;
            if let LinkAttribute::LinkInfo(infos) = link_attr {
                for info in infos {
                    match info {
                        LinkInfo::Kind(InfoKind::Xfrm) => is_xfrm = true,
                        LinkInfo::Kind(InfoKind::Other(desc)) => {
                            if desc.ne("xfrm") {
                                info!("get interface {}, but it is a {:?} device", name, desc)
                            } else {
                                is_xfrm = true
                            }
                        }
                        LinkInfo::Kind(kind) => {
                            info!("get interface {}, but it is a {:?} device", name, kind)
                        }
                        LinkInfo::Data(InfoData::Xfrm(info_data)) => {
                            let mut info_data_iter = info_data.iter();
                            while let Some(InfoXfrm::IfId(if_id)) = info_data_iter.next() {
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
            if !is_xfrm {
                return Err(GetResults::TypeNotMatch);
            };
        }
        // check the attrs of the interface
        // leave it a single match, maybe we are going to match more attrs in the future
        #[allow(clippy::single_match)]
        for link_attr in link.attributes.iter() {
            match link_attr {
                // check link state
                LinkAttribute::OperState(state) => match state {
                    State::Up | State::Unknown => {
                        debug!("interface {} operstate is {:?}", name, state)
                    }
                    _ => {
                        info!(
                            "get interface {}, but its state was not the expected value ({:?})",
                            name, state
                        );
                        return Err(GetResults::IfOperStateNotMatch);
                    }
                },
                _ => (),
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
  master_dev: Option<String>,
) -> Result<(), Error> {
  match netns_name {
    None => new_xfrm(interface, if_id, alt_names, master_dev).await,
    Some(my_netns_name) => {
      new_xfrm(interface.clone(), if_id, alt_names, master_dev).await?;
      if let Err(e) = move_to_netns(&interface, &my_netns_name).await {
        error!(
          "Failed to move {} to netns: {} [Trying to delete it from netns {} and try again]",
          interface, e, my_netns_name,
        );
        let _ = del_in_netns(Some(my_netns_name.clone()), interface.clone()).await;
        if let Err(e) = move_to_netns(&interface, &my_netns_name).await {
          error!(
            "Failed to move {} to netns: {} [Deleting it as a temporary solution...]",
            interface, e
          );
          del(interface).await?;
        }
      }
      Ok(())
    }
  }
}

// wrapper to delete an interface by its name in a given netns
pub async fn del_in_netns(netns_name: Option<String>, interface: String) -> Result<(), Error> {
    match netns_name {
        None => del(interface).await,
        #[allow(clippy::unit_arg)]
        Some(my_netns_name) => netns::operate_in_netns(my_netns_name, del(interface))
            .await?
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
