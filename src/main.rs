use crate::configuration::Configuration;
use clap::Parser;
use libc::{kill, pid_t, SIGHUP};
use linkme::distributed_slice;
use log::{info, LevelFilter};
use std::path::Path;

mod configuration;
mod docker;
mod nginx;

#[distributed_slice]
pub static STARTERS: [fn(configuration: &Configuration) -> anyhow::Result<()>];

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config_file = args
        .config
        .or_else(|| Some("./config.toml".to_string()))
        .ok_or(anyhow::anyhow!("No config file specified."))?;

    if !Path::new(&config_file).exists() {
        return Err(anyhow::anyhow!(
            "Config file {} does not exist",
            config_file
        ));
    }

    let config = Configuration::new(&config_file)?;

    env_logger::builder().filter_level(LevelFilter::Info).init();

    for starter in STARTERS {
        starter(&config)?;
    }

    println!("Done!");

    tokio::time::sleep(std::time::Duration::from_secs(1000)).await;

    Ok(())
}
