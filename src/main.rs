use futures::future::join_all;
use futures::FutureExt;
use std::time::Duration;

use clap::Parser;
use env_logger::Builder;
use eyre::{Error, WrapErr};
use log::LevelFilter;
use log::{error, info, warn};
use misc::synthesize;
use syslog::{BasicLogger, Facility, Formatter3164};
use tokio::time::timeout;

mod babeld;
mod executor;
mod interface;
mod misc;
mod netns;

// pkg name
const MYSELF: &str = "swan-updown";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct SwanUpdown {
    // needed by [all],
    /// The prefix of the created interfaces
    #[arg(short, long, value_name = "prefix", default_value = "swan")]
    prefix: String,
    // needed by [interface],
    /// Optional network namespace to move interfaces into
    #[arg(short, long, value_name = "netns")]
    netns: Option<String>,
    // needed by [interface],
    /// Optional master device to assign interfaces to
    #[arg(short, long, value_name = "master")]
    master: Option<String>,

    // needed by [babeld],
    /// The path of the babeld socket
    /// (This enables adding/deleting interfaces to babeld)
    #[arg(short, long, value_name = "babeld_sock")]
    babeld_sock: Option<String>,
    /// The babeld config for the interfaces
    #[arg(long, value_name = "babeld_conf", default_value = "type tunnel link-quality true")]
    babeld_conf: String,

    // for debug
    /// Send log to stdout, otherwise the log will be sent to syslog
    #[arg(long, action = clap::ArgAction::SetTrue)]
    to_stdout: bool,
    /// Set it multiple times to increase log level, [0: Error, 1: Warn, 2: Info, 3: Debug]
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

async fn main2() -> Result<(), Error> {
    let args = SwanUpdown::parse();

    // debug level
    let my_loglevel = match args.debug {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    fn init_env_logger(my_loglevel: LevelFilter) {
        Builder::new().filter_level(my_loglevel).init();
    }

    if args.to_stdout {
        // use stdout
        init_env_logger(my_loglevel);
    } else {
        // use syslog
        let formatter = Formatter3164 {
            facility: Facility::LOG_USER,
            hostname: None,
            process: MYSELF.into(),
            pid: 42,
        };
        match syslog::unix(formatter) {
            Ok(logger) => {
                log::set_boxed_logger(Box::new(BasicLogger::new(logger)))?;
                log::set_max_level(my_loglevel);
            }
            // fallback to stdout if syslog goes wrong
            Err(e) => {
                init_env_logger(my_loglevel);
                warn!("failed to create syslog logger, now swtich to env_logger: {}", e);
            }
        }
    }

    let trigger = misc::find_env("PLUTO_VERB")?;
    // TODO: what if IF_ID_IN and IF_ID_OUT are different?
    let conn_if_id: u32 = misc::find_env("PLUTO_IF_ID_IN")?
        .parse::<u32>()
        .wrap_err("parse if_id failed")?;
    let interface_name = synthesize(&args.prefix, conn_if_id);
    let id_pair = format!(
        "Me: {} <-> Peer: {} [{}]",
        misc::find_env("PLUTO_MY_ID")?,
        misc::find_env("PLUTO_PEER_ID")?,
        misc::find_env("PLUTO_UNIQUEID")?
    );
    let ip_pair = format!(
        "Me: {} <-> Peer: {} [{}]",
        misc::find_env("PLUTO_ME")?,
        misc::find_env("PLUTO_PEER")?,
        misc::find_env("PLUTO_UNIQUEID")?
    );
    let alt_names: Vec<&str> = vec![&id_pair, &ip_pair];

    let mut tasks = Vec::new();
    tasks.push(
        executor::interface_updown(
            &trigger,
            args.netns.clone(),
            interface_name.clone(),
            conn_if_id,
            &alt_names,
            args.master,
        )
        .boxed(),
    );
    info!("creating corresponding XFRM interface");
    // whether we communicate the babeld socket
    if let Some(sock_path) = args.babeld_sock {
        tasks.push(executor::babeld_updown(
            &trigger,
            sock_path,
            interface_name.clone(),
            args.babeld_conf
            ).boxed());
        info!("adding corresponding XFRM interface to babeld");
    }

    for res in timeout(Duration::from_secs(60), join_all(tasks))
        .await
        .wrap_err("It takes too long to complete")?
    {
        res?
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = main2().await {
        error!("main: {:?}", e);
    }
}
