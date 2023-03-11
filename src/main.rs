use clap::Parser;
use futures::future::{join_all, FutureExt};
use log::LevelFilter;
use log::{error, info, warn};
use misc::synthesize;
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;
use syslog::{BasicLogger, Facility, Formatter3164};
use tokio::time::timeout;

mod babeld;
mod executor;
mod interface;
mod misc;
mod netns;

const MYSELF: &str = "swan-updown";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct SwanUpdown {
    /// things available: [interface, babeld], can be specified multiple times
    #[arg(long, value_name = "enable")]
    enable: Vec<String>,

    /// needed by [all],        the prefix of the created interfaces, default to [swan]
    #[arg(short, long, value_name = "prefix")]
    prefix: Option<String>,
    /// needed by [interface],  Optional network namespace to move interfaces into
    #[arg(short, long, value_name = "netns")]
    netns: Option<String>,
    /// needed by [babeld],     the socket path of the babeld control socket
    #[arg(short, long, value_name = "prefix")]
    babeld_ctl: Option<String>,

    // for debug
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
    let interface_name = synthesize(&if_prefix, conn_if_id);

    let mut tasks = Vec::new();
    let enabled_modules = BTreeSet::from_iter(args.enable);
    for enabled in enabled_modules {
        if enabled == "interface" {
            tasks.push(
                executor::interface_updown(
                    &trigger,
                    args.netns.clone(),
                    interface_name.clone(),
                    conn_if_id,
                )
                .boxed(),
            );
            info!("enabling module interface");
        } else if enabled == "babeld" {
            match &args.babeld_ctl {
                Some(socket_path) => {
                    tasks.push(
                        executor::babeld_updown(&trigger, &interface_name, Path::new(socket_path))
                            .boxed(),
                    );
                    info!("enabling module babeld");
                }
                None => warn!(
                    "Module babeld enabled but babeld control socket not specified, skipping.."
                ),
            };
        }
    }

    for result in timeout(Duration::from_secs(60), join_all(tasks))
        .await
        .map_err(|e| error!("It takes too long to complete: {}", e))?
        .into_iter()
    {
        result?
    }

    Ok(())
}
