use std::time::Duration;

use clap::Parser;
use log::LevelFilter;
use log::info;
use misc::synthesize;
use syslog::{BasicLogger, Facility, Formatter3164};
use tokio::time::timeout;
use eyre::{Error, WrapErr};

mod executor;
mod interface;
mod misc;
mod netns;

const MYSELF: &str = "swan-updown";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct SwanUpdown {
    /*
    /// things available: [interface], can be specified multiple times
    #[arg(long, value_name = "enable")]
    enable: Vec<String>,
     */
    // needed by [all],
    /// the prefix of the created interfaces, default to [swan]
    #[arg(short, long, value_name = "prefix")]
    prefix: Option<String>,
    // needed by [interface],
    ///Optional network namespace to move interfaces into
    #[arg(short, long, value_name = "netns")]
    netns: Option<String>,
    // needed by [interface],
    /// Optional master device to assign interfaces to
    #[arg(short, long, value_name = "master")]
    master: Option<String>,

    // for debug
    /// send log to stdout, otherwise the log will be sent to syslog
    #[arg(long, action = clap::ArgAction::SetTrue)]
    to_stdout: bool,
    /// (it only applies to syslog) set it multiple time to increase log level, [0: Error, 1: Warn, 2: Info, 3: Debug]
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = SwanUpdown::parse();

    // debug level
    let my_loglevel = match args.debug {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    fn init_env_logger() {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("swan-updown=warn"),
        )
        .init();
    }

    if args.to_stdout {
        // use stdout
        init_env_logger();
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
                println!("failed to create logger, swtich to env_logger: {}", e);
                init_env_logger();
            }
        }
    }

    let if_prefix = args.prefix.unwrap_or_else(|| "swan".into());
    let trigger = misc::find_env("PLUTO_VERB")?;
    // TODO: what if IF_ID_IN and IF_ID_OUT are different?
    let conn_if_id: u32 = misc::find_env("PLUTO_IF_ID_IN")?
        .parse::<u32>()
        .wrap_err("parse if_id failed")?;
    let interface_name = synthesize(&if_prefix, conn_if_id);
    let id_pair = format!(
        "Me: {} <-> Peer: {}",
        misc::find_env("PLUTO_MY_ID")?,
        misc::find_env("PLUTO_PEER_ID")?
    );
    let ip_pair = format!(
        "Me: {} <-> Peer: {}",
        misc::find_env("PLUTO_ME")?,
        misc::find_env("PLUTO_PEER")?,
    );
    let alt_names: Vec<&str> = vec![&id_pair, &ip_pair];

    let task = executor::interface_updown(
        &trigger,
        args.netns.clone(),
        interface_name.clone(),
        conn_if_id,
        &alt_names,
        args.master,
    );
    info!("enabling module interface");

    timeout(Duration::from_secs(60), task)
      .await
      .wrap_err("It takes too long to complete")??;

    Ok(())
}
