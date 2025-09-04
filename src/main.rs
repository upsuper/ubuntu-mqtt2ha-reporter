use crate::config::Config;
use anyhow::{Context as _, Error};
use log::{info, trace};
use mimalloc::MiMalloc;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::{fs, thread};
use tokio::sync::oneshot;

mod command;
mod command_subscriber;
mod commands;
mod config;
mod discovery_publisher;
mod ha;
mod machine_id;
mod main_loop;
mod sensor;
mod sensor_publisher;
mod sensors;
mod utils;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let hostname = hostname::get().context("Failed to get hostname")?;
    let hostname: &str = hostname.to_string_lossy().into_owned().leak();
    info!("Hostname: {}", hostname);
    let machine_id = machine_id::get().context("Failed to get machine ID")?;
    let machine_id: &str = machine_id.hyphenated().to_string().leak();
    info!("Machine ID: {}", machine_id);

    info!("Reading config...");
    let config = fs::read_to_string("config.toml").context("Could not read config.toml")?;
    let config = toml::from_str::<Config>(&config).context("Could not parse config.toml")?;
    trace!("Config: {:#?}", config);

    let mut signals =
        Signals::new([SIGINT, SIGTERM]).context("Failed to initialize signal handler")?;
    let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
    thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            info!("Received signal {}, shutting down", signal);
            assert!(matches!(signal, SIGINT | SIGTERM));
            shutdown_sender.send(()).expect("Failed to send shutdown");
        }
    });

    let main_loop = main_loop::MainLoop::new(hostname, machine_id, config)?;
    main_loop.run(shutdown_receiver).await
}
