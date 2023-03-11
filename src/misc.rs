use log::{error, info, trace};
use rtnetlink::{new_connection, Handle};
use std::env;

#[inline(always)]
pub fn find_env(key: &str) -> Result<String, ()> {
    match env::var(key) {
        Ok(value) => {
            info!("Environment variable {} found: {}", key, value);
            Ok(value)
        },
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
