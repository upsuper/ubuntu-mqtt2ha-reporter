use crate::config::Config;
use crate::main_loop::StopReason;
use crate::sleep_monitor::SleepEvent;
use anyhow::{Context as _, Error, Result, anyhow};
use futures_util::{Stream, TryStreamExt as _, pin_mut};
use log::{info, trace};
use mimalloc::MiMalloc;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::pin::Pin;
use std::{fs, thread};
use tokio::select;
use tokio::sync::SetOnce;

mod command;
mod command_subscriber;
mod commands;
mod config;
mod connections;
mod discovery_publisher;
mod ha;
mod machine_id;
mod main_loop;
mod sensor;
mod sensor_publisher;
mod sensors;
mod sleep_monitor;
mod utils;

#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

struct HostInformation {
    hostname: &'static str,
    machine_id: &'static str,
    connections: Vec<(&'static str, String)>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();

    let hostname = hostname::get().context("Failed to get hostname")?;
    let hostname: &str = hostname.to_string_lossy().into_owned().leak();
    info!("Hostname: {}", hostname);
    let machine_id = machine_id::get().context("Failed to get machine ID")?;
    let machine_id: &str = machine_id.hyphenated().to_string().leak();
    info!("Machine ID: {}", machine_id);
    let connections = connections::get_connections().context("Failed to get connections")?;
    info!(
        "Connections: {}",
        connections::Display(connections.as_slice()),
    );
    let host_info = HostInformation {
        hostname,
        machine_id,
        connections,
    };

    info!("Reading config...");
    let config = fs::read_to_string("config.toml").context("Could not read config.toml")?;
    let config = toml::from_str::<Config>(&config).context("Could not parse config.toml")?;
    trace!("Config: {:#?}", config);

    let mut signals =
        Signals::new([SIGINT, SIGTERM]).context("Failed to initialize signal handler")?;
    static SHUTDOWN: SetOnce<()> = SetOnce::const_new();
    thread::spawn(move || {
        if let Some(signal) = signals.forever().next() {
            info!("Received signal {}, shutting down", signal);
            assert!(matches!(signal, SIGINT | SIGTERM));
            SHUTDOWN.set(()).expect("Failed to set shutdown");
        }
    });

    let sleep_monitor = sleep_monitor::SleepMonitor::new().await?;
    let sleep_events = sleep_monitor.start_monitoring().await?;
    pin_mut!(sleep_events);

    let main_loop = main_loop::MainLoop::new(host_info, config)?;
    loop {
        let stop = async {
            select! {
                _ = wait_for_sleep_event(&mut sleep_events, SleepEvent::PreparingSleep) => StopReason::Sleep,
                _ = SHUTDOWN.wait() => StopReason::Shutdown,
            }
        };
        {
            let _inhibitor_lock = sleep_monitor.take_inhibitor_lock().await?;
            main_loop.run(stop).await?;
        }
        if SHUTDOWN.get().is_some() {
            break;
        }
        wait_for_sleep_event(&mut sleep_events, SleepEvent::WakingUp).await?;
    }
    Ok(())
}

async fn wait_for_sleep_event<S>(
    sleep_events: &mut Pin<&mut S>,
    event: SleepEvent,
) -> Result<(), Error>
where
    S: Stream<Item = Result<SleepEvent>>,
{
    while let Some(e) = sleep_events.try_next().await? {
        if e == event {
            return Ok(());
        }
    }
    Err(anyhow!("Unexpected end of sleep events stream"))
}
