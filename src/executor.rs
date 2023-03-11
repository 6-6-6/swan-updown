use std::path::Path;
use crate::babeld;
use crate::interface;
use crate::interface::GetResults;
use log::info;

pub async fn interface_updown(
    trigger: &str,
    netns: Option<String>,
    interface_name: String,
    conn_if_id: u32,
) -> Result<(), ()> {
    // process by PLUTO_VERB
    if trigger.starts_with("up-client") {
        match interface::get_in_netns(netns.clone(), interface_name.clone(), conn_if_id).await {
            Ok(()) => Ok(()),
            Err(GetResults::NotFound) => interface::add_to_netns(netns, interface_name, conn_if_id).await,
            Err(_) => {
                interface::del_in_netns(netns.clone(), interface_name.clone()).await?;
                interface::add_to_netns(netns, interface_name, conn_if_id).await
            }
        }
    } else if trigger.starts_with("down-client") {
        match interface::get_in_netns(netns.clone(), interface_name.clone(), conn_if_id).await {
            Err(GetResults::NotFound) => Ok(()),
            _ => interface::del_in_netns(netns, interface_name).await,
        }
    } else {
        info!("[interface_updown] No action is taken for PLUTO_VERB {}", trigger);
        Ok(())
    }
}

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
        info!("[babeld_updown] No action is taken for PLUTO_VERB {}", trigger);
        Ok(())
    }
}
