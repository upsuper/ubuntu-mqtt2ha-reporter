use crate::config::{Config, Mqtt};
use crate::publisher::Publisher;
use crate::sensors::create_sensors;
use crate::utils::snake_case::make_snake_case;
use anyhow::{anyhow, Context, Error};
use log::{info, trace, warn};
use mimalloc::MiMalloc;
use rumqttc::{AsyncClient, MqttOptions};
use std::fs;
use std::time::Duration;
use tokio::time::{interval, sleep, timeout, MissedTickBehavior};
use tokio::{select, task};

mod commands;
mod config;
mod ha;
mod machine_id;
mod publisher;
mod sensor;
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

    let topic_base = format!("{}/{}", config.mqtt.base_topic, make_snake_case(hostname));
    let sensors = create_sensors(&topic_base)?;

    let options = build_mqtt_options(hostname, &config.mqtt)?;
    let (client, mut event_loop) = AsyncClient::new(options, 10);

    let publishing = task::spawn(async move {
        let publisher = Publisher {
            hostname,
            machine_id,
            config: &config.mqtt,
            client: &client,
            sensors: &sensors,
        };
        if let Err(e) = publisher.publish_discovery().await {
            return e.context("Failed to publish discovery");
        }
        // Wait for a few seconds before publishing the first status.
        sleep(Duration::from_secs(5)).await;

        let interval_duration =
            Duration::from_secs(u64::from(config.daemon.interval_in_minutes) * 60);
        let mut interval = interval(interval_duration);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        // Don't let publishing breach 80% of interval.
        let timeout_duration = interval_duration * 4 / 5;

        loop {
            interval.tick().await;
            match timeout(timeout_duration, publisher.publish_status()).await {
                Ok(()) => {}
                // Ignore timeout.
                Err(_) => warn!("Timeout publishing"),
            }
        }
    });

    let event_loop = task::spawn(async move {
        loop {
            if let Err(e) = event_loop.poll().await {
                break anyhow!(e).context("Failed to poll event loop");
            }
        }
    });

    select! {
        e = publishing => Err(e.context("Failed to join publishing task")?),
        e = event_loop => Err(e.context("Failed to join event loop")?),
    }
}

fn build_mqtt_options(hostname: &str, config: &Mqtt) -> Result<MqttOptions, Error> {
    use rumqttc::Transport;

    let mut options = MqttOptions::new(hostname, &config.hostname, config.port);
    options.set_keep_alive(Duration::from_secs(config.keep_alive));
    if config.tls {
        options.set_transport(match &config.tls_ca_cert {
            Some(path) => {
                let ca = fs::read(path).context("Could not read CA certificate file")?;
                Transport::tls(ca, None, None)
            }
            None => Transport::tls_with_default_config(),
        });
    }
    if let Some(username) = &config.username {
        options.set_credentials(username, config.password.as_deref().unwrap_or(""));
    }
    Ok(options)
}
