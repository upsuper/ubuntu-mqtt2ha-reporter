use crate::ha::discovery::{Device, HaSensorDiscovery};
use crate::sensor::Sensor;
use crate::sensors::Sensors;
use crate::utils::snake_case::make_snake_case;
use anyhow::{Context as _, Error};
use log::debug;
use rumqttc::{AsyncClient, QoS};

pub async fn publish_discovery(
    client: &AsyncClient,
    availability_topic: &str,
    discovery_prefix: &str,
    hostname: &str,
    machine_id: &str,
    sensors: &Sensors,
) -> Result<(), Error> {
    let Sensors {
        monitor_sensor,
        cpu_sensor,
        memory_sensor,
        disk_sensor,
        load_sensor,
        net_sensor,
        apt_sensor,
        reboot_sensor,
    } = sensors;
    let hostname_snake = make_snake_case(hostname);
    let device = Device {
        name: Some(hostname),
        identifiers: &[machine_id],
    };
    let publisher = DiscoveryPublisher {
        client,
        availability_topic,
        discovery_prefix,
        hostname_snake: &hostname_snake,
        device: &device,
    };

    tokio::try_join!(
        publisher.publish_sensor(monitor_sensor),
        publisher.publish_sensor(cpu_sensor),
        publisher.publish_sensor(memory_sensor),
        publisher.publish_sensor(disk_sensor),
        publisher.publish_sensor(load_sensor),
        publisher.publish_sensor(net_sensor),
        publisher.publish_sensor(apt_sensor),
        publisher.publish_sensor(reboot_sensor),
    )?;

    Ok(())
}

struct DiscoveryPublisher<'a> {
    client: &'a AsyncClient,
    availability_topic: &'a str,
    discovery_prefix: &'a str,
    hostname_snake: &'a str,
    device: &'a Device<'a>,
}

impl<'a> DiscoveryPublisher<'a> {
    async fn publish_sensor<S: Sensor>(&self, sensor: &S) -> Result<(), Error> {
        for item in sensor.discovery_data() {
            let sensor_id = format!("{}_{}", self.hostname_snake, item.id);
            let discovery_topic = {
                let prefix = self.discovery_prefix;
                let sensor_type = if item.binary {
                    "binary_sensor"
                } else {
                    "sensor"
                };
                format!("{prefix}/{sensor_type}/{sensor_id}/config")
            };
            let payload = {
                let discovery = HaSensorDiscovery::new(
                    &sensor_id,
                    sensor.topic(),
                    &item,
                    self.availability_topic,
                    self.device,
                );
                serde_json::to_string(&discovery).unwrap()
            };
            debug!("Publishing {} to {}", payload, discovery_topic);
            self.client
                .publish(discovery_topic, QoS::AtLeastOnce, true, payload)
                .await
                .with_context(|| format!("Failed to publish discovery for sensor {}", item.id))?;
        }

        Ok(())
    }
}
