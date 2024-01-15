use crate::interface::GetResults;
use futures::TryStreamExt;
use log::{error, info, trace};
use rtnetlink::{new_connection, Handle};
use std::env;

#[inline(always)]
pub async fn get_index_by_name(
    handle: &mut Handle,
    ifname: Option<String>,
) -> Result<u32, GetResults> {
    // get interface's index by its name
    match ifname {
        // si non
        None => Ok(0),
        Some(name) => {
            let mut links = handle.link().get().match_name(name.clone()).execute();
            // I personally cannot imagine I will need a loop to process a single match
            //   correct me if i am wrong.
            if let Some(link) = links.try_next().await.map_err(|_| GetResults::NotFound)? {
                let idx = link.header.index;
                info!("interface {} found, index: {}", name, idx);
                Ok(idx)
            } else {
                error!("Cannot find interface {}", name);
                Err(GetResults::NotFound)
            }
        }
    }
}

#[inline(always)]
pub fn find_env(key: &str) -> Result<String, ()> {
    match env::var(key) {
        Ok(value) => {
            info!("Environment variable {} found: {}", key, value);
            Ok(value)
        }
        Err(e) => {
            error!("Environment variable {} not found: {}, exiting...", key, e);
            Err(())
        }
    }
}

#[inline(always)]
pub fn netlink_handle() -> Result<Handle, ()> {
    // TODO: proper log message
    trace!("get netlink handle");
    let (connection, handle, _) = new_connection().map_err(|e| error!("{}", e))?;
    tokio::spawn(connection);
    Ok(handle)
}

#[inline(always)]
pub fn synthesize(if_prefix: &str, if_id: u32) -> String {
    // use big-endian for better readability
    format!("{}{}", if_prefix, hex::encode(if_id.to_be_bytes()))
}
