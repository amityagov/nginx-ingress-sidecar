use crate::settings::NginxSettings;
use crate::template::{render_template, Template};
use libc::{kill, pid_t, SIGHUP};
use log::info;
use serde::Serialize;
use std::panic::UnwindSafe;
use std::path::{Path, PathBuf};
use std::{fs, panic};

#[derive(Debug, Serialize, Default)]
struct ServiceRenderContext {
    path: String,
    name: String,
    upstreams: Vec<Upstream>,
    listen_port: u16,
    ssl: bool,
    server_name: String,
}

impl Template for ServiceRenderContext {
    const NAME: &'static str = "service";

    const TEMPLATE: &'static str = include_str!("../templates/service.tmpl");
}

#[derive(Debug, Serialize)]
struct Upstream {
    address: String,
    port: u16,
    weight: u8,
}

pub enum ServiceOperation {
    Remove,
    Add,
}

pub fn apply_operations(_operations: Vec<ServiceOperation>) -> anyhow::Result<()> {
    Ok(())
}

pub fn get_nginx_pid<T: AsRef<Path>>(pid_file_path: T) -> anyhow::Result<i32> {
    Ok(fs::read_to_string(pid_file_path)?.trim().parse::<i32>()?)
}

pub fn send_nginx_reload_signal<P: Into<pid_t> + UnwindSafe>(pid: P) -> anyhow::Result<()> {
    panic::catch_unwind(|| unsafe {
        kill(pid.into(), SIGHUP);
    })
    .map_err(|_| anyhow::anyhow!("reloading nginx failed"))?;

    info!("reloaded nginx with HUP");
    Ok(())
}

pub fn reload_nginx<T: AsRef<Path>>(path: T) -> anyhow::Result<()> {
    let pid = get_nginx_pid(path)?;
    send_nginx_reload_signal(pid)?;
    Ok(())
}

pub fn save_service_template_and_reload_nginx(settings: &NginxSettings) -> anyhow::Result<()> {
    let context = ServiceRenderContext {
        ..Default::default()
    };

    let template = render_template(&context)?;
    fs::write(&settings.servers_path, template)?; // TODO, filename
    Ok(())
}

pub fn enumerate_existing_services(settings: &NginxSettings) -> anyhow::Result<Vec<PathBuf>> {
    let pattern = Path::new(settings.servers_path.as_str())
        .join("**/nis_*.conf")
        .to_string_lossy()
        .to_string();

    Ok(glob::glob(&pattern)?.filter_map(Result::ok).collect())
}
