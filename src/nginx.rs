use libc::{kill, pid_t, SIGHUP};
use log::info;
use serde::Serialize;
use std::panic::UnwindSafe;
use std::path::Path;
use std::{fs, panic};
use tinytemplate::TinyTemplate;

static SERVICE_TEMPLATE: &'static str = include_str!("../templates/service.tmpl");

#[derive(Debug, Serialize)]
struct ServiceRenderContext {
    path: String,
    name: String,
    upstreams: Vec<Upstream>,
    listen_port: u16,
    ssl: bool,
    server_name: String,
}

#[derive(Debug, Serialize)]
struct Upstream {
    address: String,
    port: u16,
    weight: u8,
}

fn render_template<T: Serialize>(context: &T) -> anyhow::Result<String> {
    let mut tt = TinyTemplate::new();
    tt.add_template("service", SERVICE_TEMPLATE)?;
    Ok(tt.render("service", &context)?)
}

pub fn get_nginx_pid<T: AsRef<Path>>(pid_file_path: T) -> anyhow::Result<u32> {
    Ok(fs::read_to_string(pid_file_path)?.trim().parse::<u32>()?)
}

pub fn reload_nginx<P: Into<pid_t> + UnwindSafe>(pid: P) -> anyhow::Result<()> {
    panic::catch_unwind(|| unsafe {
        kill(pid.into(), SIGHUP);
    })
    .map_err(|_| anyhow::anyhow!("reloading nginx failed"))?;

    info!("reloaded nginx with HUP");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_service_template_is_ok() -> anyhow::Result<()> {
        let context = ServiceRenderContext {
            path: "/".to_string(),
            name: "service".to_string(),
            listen_port: 80,
            server_name: "service.com".to_string(),
            ssl: false,
            upstreams: vec![Upstream {
                address: "192.168.200.26".to_string(),
                port: 8000,
                weight: 10,
            }],
        };

        println!("{:}", render_template(&context)?);
        Ok(())
    }
}
