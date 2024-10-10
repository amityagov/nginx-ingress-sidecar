use crate::configuration::{Configuration, DockerConfiguration};
use crate::nginx::{get_nginx_pid, send_nginx_reload_signal};
use crate::settings::{DockerSettings, NginxSettings, Settings};
use crate::STARTERS;
use anyhow::anyhow;
use bollard::container::ListContainersOptions;
use bollard::models::{ContainerSummary, EventActor, EventMessage, EventMessageTypeEnum};
use bollard::Docker;
use futures_util::StreamExt;
use linkme::distributed_slice;
use log::{error, info};
use std::collections::HashMap;

const SERVICE_MARKER_LABEL: &str = "service";
const SERVICE_HOST_LABEL: &str = "service-host";
const SERVICE_PROTO_LABEL: &str = "service-protocol";
const SERVICE_PORT_LABEL: &str = "service-port";

struct Config {
    nginx: NginxSettings,
    label_prefix: Option<String>,
}

impl Config {
    fn new(nginx: &NginxSettings, docker: DockerSettings) -> Self {
        Self {
            nginx: nginx.clone(),
            label_prefix: docker.label_prefix,
        }
    }

    pub fn with_label_prefix(&self, label: &str) -> String {
        if let Some(label_prefix) = &self.label_prefix {
            let mut label_prefix = label_prefix.clone();
            label_prefix.push_str(label);
            return label_prefix;
        }

        label.to_string()
    }
}

#[distributed_slice(STARTERS)]
pub fn start(settings: &Settings) -> anyhow::Result<()> {
    let docker = settings.docker.clone();
    if let Some(docker) = docker {
        let config = Config::new(&settings.nginx, docker);

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

async fn start_task(config: Config) -> anyhow::Result<()> {
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

async fn initial_sync(config: &Config, docker: &Docker) -> anyhow::Result<()> {
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
    config: &Config,
    docker: &Docker,
) -> anyhow::Result<()> {
    if event.typ != Some(EventMessageTypeEnum::CONTAINER) {
        return Ok(());
    }

    if let Some(actor) = event.actor {
        if let EventActor { id: Some(id), .. } = actor {
            match &event.action {
                Some(action) if action == "stop" => {
                    remove_container_from_nginx(id, config, &docker).await?;
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
    config: &Config,
    docker: &Docker,
) -> anyhow::Result<()> {
    let container = find_container(id, &docker).await?;
    if let Some(container) = container {
        if let Some(labels) = container.labels {
            let _descriptor = ServiceConfiguration::from_labels(&config, &labels)?;
            let _name = _descriptor.name;
        }
    }

    Ok(())
}

async fn add_container_to_nginx(
    id: String,
    config: &Config,
    docker: &Docker,
) -> anyhow::Result<()> {
    let container = find_container(id, &docker).await?;

    if let Some(container) = container {
        process_add_container_to_nginx(container, &config, &docker).await?;
    }

    Ok(())
}

async fn find_container<T: Into<String>>(
    id: T,
    docker: &Docker,
) -> anyhow::Result<Option<ContainerSummary>> {
    let mut filters = HashMap::new();
    filters.entry("id".to_string()).or_insert(vec![id.into()]);

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
    config: &Config,
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
    fn from_labels(config: &Config, labels: &HashMap<String, String>) -> anyhow::Result<Self> {
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
