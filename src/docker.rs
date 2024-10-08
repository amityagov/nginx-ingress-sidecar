use std::borrow::Cow;
use crate::configuration::{Configuration, DockerConfiguration};
use crate::STARTERS;
use anyhow::anyhow;
use bollard::container::ListContainersOptions;
use bollard::models::{ContainerSummary, EventActor, EventMessage, EventMessageTypeEnum};
use bollard::{container, Docker};
use futures_util::StreamExt;
use linkme::distributed_slice;
use log::{error, info};
use std::collections::HashMap;
use clap::builder::Str;
use crate::nginx::get_nginx_pid;

const SERVICE_MARKER_LABEL: &str = "service";
const SERVICE_HOST_LABEL: &str = "service-host";
const SERVICE_PROTO_LABEL: &str = "service-protocol";
const SERVICE_PORT_LABEL: &str = "service-port";

#[distributed_slice(STARTERS)]
pub fn start(configuration: &Configuration) -> anyhow::Result<()> {
    let docker_configuration = configuration.docker.clone();
    if let Some(config) = docker_configuration {
        tokio::spawn(async move {
            match start_task(config).await {
                Ok(_) => {}
                Err(error) => {
                    println!("Failed to start docker service: {}", error);
                }
            }
        });
    }

    Ok(())
}

async fn start_task(config: DockerConfiguration) -> anyhow::Result<()> {
    info!("Starting listening docker containers changes");
    let docker = Docker::connect_with_defaults()?;

    initial_sync(&config, &docker).await?;

    let mut stream = docker.events::<String>(None);
    loop {
        if let Some(event) = stream.next().await {
            match event {
                Ok(event) => {
                    process_event(event, &config, &docker).await?;
                }
                Err(error) => {
                    error!("{}", error);
                }
            }
        } else {
            break;
        }
    }

    info!("exit docker event loop");
    Ok(())
}
async fn initial_sync(config: &DockerConfiguration, docker: &Docker) -> anyhow::Result<()> {
    let containers = docker
        .list_containers::<String>(Some(ListContainersOptions {
            all: true,
            ..Default::default()
        }))
        .await?;

    for container in containers {
        process_add_container_to_nginx(container, config, &docker).await?;
    }

    Ok(())
}

async fn process_event(
    event: EventMessage,
    config: &DockerConfiguration,
    docker: &Docker,
) -> anyhow::Result<()> {
    if event.typ != Some(EventMessageTypeEnum::CONTAINER) {
        return Ok(());
    }

    if let Some(actor) = event.actor {
        if let EventActor { id: Some(id), .. } = actor {
            match &event.action {
                Some(action) if action == "stop" => {
                    remove_container_from_nginx(id, &config, &docker).await?;
                }
                Some(action) if action == "start" => {
                    add_container_to_nginx(id, &config, &docker).await?;
                }
                _ => return Ok(()),
            }
        }
    }
    Ok(())
}

async fn remove_container_from_nginx(
    id: String,
    config: &DockerConfiguration,
    docker: &Docker,
) -> anyhow::Result<()> {
    let container = find_container(id.into(), &docker).await?;
    if let Some(container) = container {
        if let Some(labels) = container.labels {
            let descriptor = ServiceConfiguration::from_labels(config, &labels)?;
        }
    }

    Ok(())
}

async fn add_container_to_nginx(
    id: String,
    config: &DockerConfiguration,
    docker: &Docker,
) -> anyhow::Result<()> {
    let container = find_container(id.into(), &docker).await?;

    if let Some(container) = container {
        process_add_container_to_nginx(container, &config, &docker).await?;
    }

    Ok(())
}

async fn find_container(id: Cow<String>, docker: &Docker) -> anyhow::Result<Option<ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.entry("id".to_string()).or_insert(vec![id.to_string()]);

    let containers = docker
        .list_containers::<String>(Some(ListContainersOptions {
            filters,
            all: true,
            ..Default::default()
        }))
        .await?;
    if containers.len() == 0 {
        return Ok(None);
    }

    if containers.len() > 1 {
        return Err(anyhow!("More than 1 container found"));
    }

    Ok(Some(containers[0].clone()))
}

async fn process_add_container_to_nginx(
    container: ContainerSummary,
    config: &DockerConfiguration,
    _docker: &Docker,
) -> anyhow::Result<()> {
    if let Some(labels) = container.labels {
        if labels.contains_key(&config.with_label_prefix(SERVICE_MARKER_LABEL)) {
            let _service = ServiceConfiguration::from_labels(&config, &labels)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct ServiceConfiguration {
    name: String,
    port: Option<i16>,
    path: Option<String>,
    host: String,
}

impl ServiceConfiguration {
    fn from_labels(
        config: &DockerConfiguration,
        labels: &HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            name: labels
                .get(&config.with_label_prefix(SERVICE_MARKER_LABEL))
                .map(|value| value.to_string())
                .ok_or(anyhow!("missing service name"))?,
            host: labels
                .get(&config.with_label_prefix(SERVICE_HOST_LABEL))
                .map(|value| value.to_string())
                .ok_or(anyhow!("missing service host"))?,
            port: labels
                .get(&config.with_label_prefix(SERVICE_PORT_LABEL))
                .map(|value| value.parse::<i16>())
                .transpose()?,
            path: labels
                .get(&config.with_label_prefix(SERVICE_PORT_LABEL))
                .map(|value| value.to_string()),
        })
    }
}
