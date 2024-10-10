use std::ops::Deref;
use std::sync::Arc;
use crate::configuration::Configuration;

#[derive(Debug, Clone)]
pub struct DockerSettings {
    pub label_prefix: Option<String>,
}

impl DockerSettings {
    pub fn new(configuration: &Configuration) -> Option<Self> {
        configuration.docker.clone().map(|docker| Self {
            label_prefix: docker.label_prefix.clone()
        })
    }
}

#[derive(Debug, Clone)]
pub struct NginxSettings {
    pub pid_file_path: String,
    pub servers_path: String,
}

impl NginxSettings {
    pub fn new(configuration: &Configuration) -> Self {
        Self {
            servers_path: configuration.servers_path.clone(),
            pid_file_path: configuration.nginx_pid_file.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    inner: Arc<Inner>,
}

impl Deref for Settings {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct Inner {
    pub nginx: NginxSettings,
    pub docker: Option<DockerSettings>,
}

impl Settings {
    pub fn new(configuration: &Configuration) -> Self {
        Self {
            inner: Arc::new(Inner {
                nginx: NginxSettings::new(&configuration),
                docker: DockerSettings::new(&configuration),
            })
        }
    }
}