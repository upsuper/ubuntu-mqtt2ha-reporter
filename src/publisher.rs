use crate::config::Mqtt;
use crate::ha::discovery::{Device, HaSensorDiscovery};
use crate::sensor::Sensor;
use crate::sensors::Sensors;
use crate::utils::snake_case::make_snake_case;
use anyhow::{Context, Error};
use log::{debug, error};
use rumqttc::{AsyncClient, QoS};
use std::any::type_name;

pub struct Publisher<'a> {
    pub hostname: &'a str,
    pub machine_id: &'a str,
    pub config: &'a Mqtt,
    pub client: &'a AsyncClient,
    pub availability_topic: &'a str,
    pub sensors: &'a Sensors,
}

impl<'a> Publisher<'a> {
    pub async fn publish_discovery(&self) -> Result<(), Error> {
        let Sensors {
            monitor_sensor,
            cpu_sensor,
            memory_sensor,
            disk_sensor,
            load_sensor,
            net_sensor,
            apt_sensor,
            reboot_sensor,
        } = self.sensors;
        let hostname_snake = make_snake_case(self.hostname);
        let device = Device {
            name: Some(self.hostname),
            identifiers: &[self.machine_id],
        };
        let publisher = DiscoveryPublisher {
            client: self.client,
            availability_topic: self.availability_topic,
            discovery_prefix: &self.config.discovery_prefix,
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

    pub async fn publish_status(&self) {
        let Sensors {
            monitor_sensor,
            cpu_sensor,
            memory_sensor,
            disk_sensor,
            load_sensor,
            net_sensor,
            apt_sensor,
            reboot_sensor,
        } = self.sensors;

        tokio::join!(
            self.publish_payload(monitor_sensor),
            self.publish_payload(cpu_sensor),
            self.publish_payload(memory_sensor),
            self.publish_payload(disk_sensor),
            self.publish_payload(load_sensor),
            self.publish_payload(net_sensor),
            self.publish_payload(apt_sensor),
            self.publish_payload(reboot_sensor),
        );
    }

    async fn publish_payload<S: Sensor>(&self, sensor: &S) {
        if let Err(e) = self.publish_payload_inner(sensor).await {
            let name = type_name::<S>();
            error!("Failed to publish for {name}: {e}");
        }
    }

    async fn publish_payload_inner<S: Sensor>(&self, sensor: &S) -> Result<(), Error> {
        let status = sensor.get_status().await.context("Failed to read status")?;
        let payload = serde_json::to_string(&status).context("Failed to serialize payload")?;
        debug!("Publishing {} to {}", payload, sensor.topic());
        self.client
            .publish(sensor.topic(), QoS::AtLeastOnce, false, payload)
            .await
            .context("Failed to publish status")?;
        Ok(())
    }
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
