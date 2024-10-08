use serde::Deserialize;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Configuration {
    inner: Arc<Inner>,
}

impl Deref for Configuration {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

#[derive(Deserialize, Debug)]
pub struct Inner {
    pub nginx_pid_file: String,
    pub servers_path: String,
    pub docker: Option<DockerConfiguration>,
}

impl Configuration {
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(file_name))
            .build()?;
        let inner = config.try_deserialize::<Inner>()?;
        Ok(Self {
            inner: Arc::new(inner),
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct DockerConfiguration {
    label_prefix: Option<String>,
}

impl DockerConfiguration {
    pub fn with_label_prefix(&self, label: &str) -> String {
        if let Some(label_prefix) = &self.label_prefix {
            let mut label_prefix = label_prefix.clone();
            label_prefix.push_str(label);
            return label_prefix;
        }

        label.to_string()
    }
}
