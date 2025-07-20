use crate::commands::create_commands;
use crate::config::{Config, Mqtt};
use crate::sensors::create_sensors;
use crate::utils::snake_case::make_snake_case;
use anyhow::{anyhow, Context as _, Error};
use futures_util::TryFutureExt;
use log::{debug, info, trace, warn};
use mimalloc::MiMalloc;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, Outgoing, QoS};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use std::time::Duration;
use std::{fs, thread};
use tokio::select;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{interval, sleep, timeout, MissedTickBehavior};

mod command;
mod command_subscriber;
mod commands;
mod config;
mod discovery_publisher;
mod ha;
mod machine_id;
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

    let topic_base = format!("{}/{}", config.mqtt.base_topic, make_snake_case(hostname));
    let sensors = create_sensors(&topic_base)?;
    let commands = create_commands(&topic_base);
    let availability_topic = format!("{topic_base}/availability");

    let options = build_mqtt_options(hostname, &config.mqtt)?;
    let (client, mut event_loop) = AsyncClient::new(options, 10);
    let (msg_sender, mut msg_receiver) = mpsc::channel(8);
    let mut event_loop = tokio::spawn(async move {
        loop {
            let event = event_loop
                .poll()
                .await
                .context("Failed to poll event loop")?;
            match event {
                Event::Incoming(Incoming::Publish(publish)) => {
                    match msg_sender.try_send(publish.topic) {
                        Ok(_) => {}
                        Err(TrySendError::Full(_)) => {
                            warn!("Dropping message due to full channel");
                        }
                        Err(TrySendError::Closed(_)) => {
                            warn!("Dropping message due to closed channel")
                        }
                    }
                }
                Event::Outgoing(Outgoing::Disconnect) => {
                    break;
                }
                _ => {}
            }
        }
        Ok::<_, Error>(())
    })
    .unwrap_or_else(|e| Err(e).context("Failed to join event loop"));

    info!("Subscribing commands...");
    let command_subscriber = command_subscriber::CommandSubscriber::new(&commands);
    command_subscriber
        .subscribe_to_commands(&client)
        .await
        .context("Failed to subscribe to commands")?;
    let handling_commands = async move {
        loop {
            let topic = msg_receiver
                .recv()
                .await
                .context("Failed to receive message")?;
            command_subscriber.handle_message(&topic).await;
        }
        #[allow(unreachable_code)]
        Ok::<_, Error>(())
    };

    info!("Publishing discovery...");
    discovery_publisher::publish_discovery(
        &client,
        &availability_topic,
        &config.mqtt.discovery_prefix,
        hostname,
        machine_id,
        &sensors,
        &commands,
    )
    .await
    .context("Failed to publish discovery")?;
    // Wait for a few seconds before publishing the first status.
    sleep(Duration::from_secs(5)).await;

    let publishing = async {
        let publisher = sensor_publisher::SensorPublisher {
            client: &client,
            sensors: &sensors,
        };
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
    };

    let sending_availability = async {
        let mut interval = interval(Duration::from_secs(60));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let publishing_online = async {
            loop {
                interval.tick().await;
                debug!("Sending online message");
                if let Err(e) = client
                    .publish(&availability_topic, QoS::AtLeastOnce, false, "online")
                    .await
                {
                    break anyhow!(e).context("Failed to publish online");
                }
            }
        };
        select! {
            e = publishing_online => return Err(e.context("Failed to publish online")),
            s = shutdown_receiver => s.context("Failed to receive shutdown signal")?,
        }
        debug!("Sending offline message");
        client
            .publish(&availability_topic, QoS::AtLeastOnce, false, "offline")
            .await
            .context("Failed to publish availability")?;
        client.disconnect().await.context("Failed to disconnect")?;
        Ok::<(), Error>(())
    };

    select! {
        r = &mut event_loop => r.context("Event loop")?,
        r = sending_availability => r.context("Sending availability")?,
        r = handling_commands => r.context("Handling commands")?,
        () = publishing => unreachable!("Publishing should never complete"),
    };
    event_loop.await
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
