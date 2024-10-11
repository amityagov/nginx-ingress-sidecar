use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct DockerConfiguration {
    pub label_prefix: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AcmeConfiguration {
    pub email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Configuration {
    pub nginx_pid_file: String,
    pub servers_path: String,
    pub docker: Option<DockerConfiguration>,
    pub acme: Option<AcmeConfiguration>,
}

impl Configuration {
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name(file_name))
            .build()?;
        Ok(config.try_deserialize::<Configuration>()?)
    }
}
