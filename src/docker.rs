use crate::configuration::{Configuration, DockerConfiguration};
use crate::STARTERS;
use bollard::container::ListContainersOptions;
use bollard::models::{ContainerSummary, EventActor, EventMessage, EventMessageTypeEnum};
use bollard::Docker;
use futures_util::StreamExt;
use linkme::distributed_slice;
use log::{error, info};
use std::collections::HashMap;

const SERVICE_MARKER_LABEL: &str = "service";
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
        process_container(container, config, &docker).await?;
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
        let EventActor { id, .. } = actor;
        if let Some(id) = id {
            let mut filters = HashMap::new();
            filters.entry("id".to_string()).or_insert(vec![id.clone()]);
            println!("with {}", id);

            let containers = docker
                .list_containers::<String>(Some(ListContainersOptions {
                    filters,
                    all: true,
                    ..Default::default()
                }))
                .await?;

            for container in containers {
                process_container(container, &config, &docker).await?;
            }
        }
    }
    Ok(())
}

async fn process_container(
    container: ContainerSummary,
    config: &DockerConfiguration,
    docker: &Docker,
) -> anyhow::Result<()> {
    println!("process container: {:?}", container.labels);

    if let Some(labels) = container.labels {
        println!("process labels: {:?}", labels);
        println!("{}", config.with_label_prefix(SERVICE_MARKER_LABEL));
        if labels.contains_key(&config.with_label_prefix(SERVICE_MARKER_LABEL)) {
            info!("found service");
            let service = ServiceConfiguration::read(&config, &labels)?;
        }
    }

    Ok(())
}

struct ServiceConfiguration {
    name: String,
    port: Option<i16>,
    path: Option<String>,
}

impl ServiceConfiguration {
    fn read(config: &DockerConfiguration, labels: &HashMap<String, String>) -> anyhow::Result<Self> {
        Ok(Self {
            name: "None".to_string(),
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
