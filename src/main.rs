use crate::configuration::Configuration;
use crate::settings::Settings;
use crate::worker::{WorkerHandle, STARTERS};
use clap::Parser;
use futures_util::future::join_all;
use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Root};
use std::path::Path;

mod acme;
mod configuration;
mod docker;
mod nginx;
mod settings;
mod template;
mod worker;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config_file = args.config.unwrap_or("./config.toml".to_string());

    if !Path::new(&config_file).exists() {
        return Err(anyhow::anyhow!(
            "Config file {} does not exist",
            config_file
        ));
    }

    let config = Configuration::new(&config_file)?;
    init_logging(&config)?;
    let settings = Settings::new(&config);

    let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
    let mut wait_handles = Vec::with_capacity(STARTERS.len());

    for starter in STARTERS {
        let cancel = shutdown_tx.subscribe();
        let (wait_tx, wait_rx) = tokio::sync::oneshot::channel();
        let handle = WorkerHandle::new(cancel, wait_tx);
        wait_handles.push(wait_rx);

        starter(&settings, handle)?;
    }

    info!("waiting exit signal");
    tokio::signal::ctrl_c().await?;

    info!("shutting down");
    shutdown_tx.send(())?;

    info!("waiting tasks to exit");
    join_all(wait_handles).await;

    info!("all tasks exited");
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
