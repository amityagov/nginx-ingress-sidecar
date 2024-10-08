use libc::{kill, pid_t, SIGHUP};
use log::info;
use std::panic::UnwindSafe;
use std::path::Path;
use std::{fs, panic};

static SERVICE_TEMPLATE: &'static str = include_str!("../templates/service.hb");

fn get_nginx_pid<T: AsRef<Path>>(pid_file_path: T) -> anyhow::Result<i32> {
    let value = fs::read_to_string(pid_file_path)?.parse::<i32>()?;
    Ok(value)
}

fn reload_nginx<P: Into<pid_t> + UnwindSafe>(pid: P) -> anyhow::Result<()> {
    panic::catch_unwind(|| unsafe {
        kill(pid.into(), SIGHUP);
    })
        .map_err(|_| anyhow::anyhow!("reloading nginx failed"))?;

    info!("reloaded nginx with HUP");
    Ok(())
}
