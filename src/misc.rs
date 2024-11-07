use std::env;

use eyre::{eyre, Error};
use futures::TryStreamExt;
use log::{error, info, trace};
use rtnetlink::{new_connection, Handle};

#[inline(always)]
pub async fn get_index_by_name(handle: &mut Handle, ifname: Option<String>) -> Result<u32, Error> {
    // get interface's index by its name
    match ifname {
        // si non
        None => Ok(0),
        Some(name) => {
            let mut links = handle.link().get().match_name(name.clone()).execute();
            // I personally cannot imagine I will need a loop to process a single match
            //   correct me if i am wrong.
            if let Some(link) = links.try_next().await? {
                let idx = link.header.index;
                info!("interface {} found, index: {}", name, idx);
                Ok(idx)
            } else {
                Err(eyre!("Cannot find interface {}", name))
            }
        }
    }
}

#[inline(always)]
pub fn find_env(key: &str) -> Result<String, env::VarError> {
    env::var(key)
        .map(|value| {
            info!("Environment variable {} found: {}", key, value);
            value
        })
        .map_err(|e| {
            error!("Environment variable {} not found: {}", key, e);
            e
        })
}

#[inline(always)]
pub fn netlink_handle() -> Result<Handle, std::io::Error> {
    trace!("get netlink handle");
    let (connection, handle, _) = new_connection()?;
    tokio::spawn(connection);
    Ok(handle)
}

#[inline(always)]
pub fn synthesize(if_prefix: &str, if_id: u32) -> String {
    // use big-endian for better readability
    format!("{}{}", if_prefix, hex::encode(if_id.to_be_bytes()))
}
