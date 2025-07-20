use crate::sensor::Sensor;
use crate::sensors::Sensors;
use anyhow::{Context, Error};
use log::{debug, error};
use rumqttc::{AsyncClient, QoS};
use std::any::type_name;

pub struct Publisher<'a> {
    pub client: &'a AsyncClient,
    pub sensors: &'a Sensors,
}

impl<'a> Publisher<'a> {
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
