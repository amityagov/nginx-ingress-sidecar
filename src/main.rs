use crate::configuration::Configuration;
use crate::settings::Settings;
use clap::Parser;
use linkme::distributed_slice;
use log::{info, warn, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use std::path::Path;

mod configuration;
mod docker;
mod nginx;
mod settings;
mod template;

#[distributed_slice]
pub static STARTERS: [fn(settings: &Settings) -> anyhow::Result<()>];

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
    init_logging(&config)?;
    let settings = Settings::new(&config);

    for starter in STARTERS {
        starter(&settings)?;
    }

    tokio::time::sleep(std::time::Duration::from_secs(1000)).await;

    Ok(())
}

fn init_logging(_: &Configuration) -> anyhow::Result<()> {
    let stdout = ConsoleAppender::builder().build();

    let mut log_builder = log4rs::Config::builder();
    log_builder = log_builder.appender(Appender::builder().build("stdout", Box::new(stdout)));

    let log_config =
        log_builder.build(Root::builder().appender("stdout").build(LevelFilter::Info))?;
    log4rs::init_config(log_config)?;

    Ok(())
}
