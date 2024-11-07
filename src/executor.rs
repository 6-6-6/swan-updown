use eyre::Error;
use log::info;

use crate::babeld::babeld_cmd;
use crate::interface;
use crate::interface::GetResults;

pub async fn interface_updown(
    trigger: &str,
    netns: Option<String>,
    interface_name: String,
    conn_if_id: u32,
    alt_names: &[&str],
    master_dev: Option<String>,
) -> Result<(), Error> {
    // process by PLUTO_VERB
    if trigger.starts_with("up-client") {
        match interface::get_in_netns(netns.clone(), interface_name.clone(), conn_if_id).await {
            Ok(()) => Ok(()),
            Err(GetResults::NotFound) => {
                interface::add_to_netns(netns, interface_name, conn_if_id, alt_names, master_dev)
                    .await
            }
            Err(_) => {
                interface::del_in_netns(netns.clone(), interface_name.clone()).await?;
                interface::add_to_netns(netns, interface_name, conn_if_id, alt_names, master_dev)
                    .await
            }
        }
    } else if trigger.starts_with("down-client") {
        match interface::get_in_netns(netns.clone(), interface_name.clone(), conn_if_id).await {
            Err(GetResults::NotFound) => Ok(()),
            _ => interface::del_in_netns(netns, interface_name).await,
        }
    } else {
        info!(
            "[interface_updown] No action is taken for PLUTO_VERB {}",
            trigger
        );
        Ok(())
    }
}

pub async fn babeld_updown(
    trigger: &str,
    babeld_sock_path: String,
    interface_name: String,
    config_line: String,
) -> Result<(), Error> {
    if trigger.starts_with("up-client") {
        babeld_cmd(
            &babeld_sock_path,
            &format!("interface {} {}", interface_name, config_line),
        )
        .await
    } else if trigger.starts_with("down-client") {
        babeld_cmd(
            &babeld_sock_path,
            &format!("flush interface {}", interface_name),
        )
        .await
    } else {
        info!(
            "[babeld_updown] No action is taken for PLUTO_VERB {}",
            trigger
        );
        Ok(())
    }
}
