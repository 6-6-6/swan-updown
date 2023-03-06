use log::{error, trace};
use rtnetlink::{new_connection, Handle};
use std::env;

#[inline(always)]
pub fn find_env(key: &str) -> Result<String, ()> {
    env::var(key).map_err(|e| error!("Environment variable {} not found: {}, exiting...", key, e))
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
    format!("{}{}", if_prefix, hex::encode(if_id.to_ne_bytes()))
}
