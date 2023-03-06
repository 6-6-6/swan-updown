use std::path::Path;

use crate::babeld;
use crate::interface;
use crate::misc;
use log::info;

// be careful when interface is configured in namespaces
// interface::move_to_netns() may affect other functions
pub async fn interface_updown(
    trigger: &str,
    netns: &Option<String>,
    interface_name: &str,
    conn_if_id: u32,
) -> Result<(), ()> {
    let handle = misc::netlink_handle()?;
    // process by PLUTO_VERB
    if trigger.starts_with("up-client") {
        interface::new_xfrm(&handle, interface_name, conn_if_id).await?;
        if let Some(netns_name) = netns {
            interface::move_to_netns(&handle, interface_name, netns_name).await?;
        }
    } else if trigger.starts_with("down-client") {
        match netns {
            Some(netns_name) => interface::del_in_netns(interface_name, netns_name).await?,
            None => interface::del(&handle, interface_name).await?,
        }
    } else {
        info!("No action is taken for PLUTO_VERB {}", trigger)
    }
    Ok(())
}

//
pub async fn babeld_updown(
    trigger: &str,
    interface_name: &str,
    socket_path: &Path,
) -> Result<(), ()> {
    // process by PLUTO_VERB
    if trigger.starts_with("up-client") {
        babeld::add_interface(socket_path, interface_name).await
    } else if trigger.starts_with("down-client") {
        babeld::del_interface(socket_path, interface_name).await
    } else {
        info!("No action is taken for PLUTO_VERB {}", trigger);
        Ok(())
    }
}
