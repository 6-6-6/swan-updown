use clap::Parser;
use log::LevelFilter;
use log::{error, info};
use misc::synthesize;
use syslog::{BasicLogger, Facility, Formatter3164};

mod interface;
mod misc;
mod netns;

const MYSELF: &str = "swan-updown";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct SwanUpdown {
    /// Optional network namespace to move interfaces into
    #[arg(short, long, value_name = "netns")]
    netns: Option<String>,
    /// the prefix of the created interfaces, default to [swan]
    #[arg(short, long, value_name = "prefix")]
    prefix: Option<String>,
    /// send log to stdout, otherwise the log will be sent to syslog
    #[arg(long, action = clap::ArgAction::SetTrue)]
    to_stdout: bool,
    /// set log level
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let args = SwanUpdown::parse();

    // debug level
    let my_loglevel = match args.debug {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    if args.to_stdout {
        // use stdout
        env_logger::init();
    } else {
        // use syslog
        let formatter = Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: MYSELF.into(),
            pid: 42,
        };
        let logger =
            syslog::unix(formatter).map_err(|e| error!("failed to create logger: {}", e))?;
        match log::set_boxed_logger(Box::new(BasicLogger::new(logger))) {
            Ok(_) => log::set_max_level(my_loglevel),
            Err(_) => return Err(()),
        };
    }

    let if_prefix = args.prefix.unwrap_or_else(|| "swan".into());
    let trigger = misc::find_env("PLUTO_VERB")?;
    // TODO: what if IF_ID_IN and IF_ID_OUT are different?
    let conn_if_id: u32 = misc::find_env("PLUTO_IF_ID_IN")?
        .parse::<u32>()
        .map_err(|e| error! {"parse if_id failed: {}", e})?;

    let handle = misc::netlink_handle()?;

    // process by PLUTO_VERB
    if trigger.starts_with("up-client") {
        interface::new_xfrm(
            &handle,
            &misc::synthesize(&if_prefix, conn_if_id),
            conn_if_id,
        )
        .await?;
        if let Some(netns_name) = args.netns {
            interface::move_to_netns(
                &handle,
                &misc::synthesize(&if_prefix, conn_if_id),
                &netns_name,
            )
            .await?;
        }
    } else if trigger.starts_with("down-client") {
        match args.netns {
            Some(netns_name) => {
                interface::del_in_netns(&synthesize(&if_prefix, conn_if_id), &netns_name).await?
            }
            None => interface::del(&handle, &synthesize(&if_prefix, conn_if_id)).await?,
        }
    } else {
        info!("No action is taken for PLUTO_VERB {}", trigger)
    }

    Ok(())
}
