use crate::ha::values::{DeviceClass, EntityCategory, StateClass};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::{Context, Error};
use nix::sys::statvfs::statvfs;
use serde::Serialize;

const ID: &str = "disk";

pub struct DiskSensor {
    topic: Box<str>,
}

impl DiskSensor {
    pub fn new(topic_base: &str) -> Self {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        DiskSensor { topic }
    }
}

impl Sensor for DiskSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        let base_discovery = SensorDiscovery::new(SensorDiscoveryInit {
            id: "",
            title: "",
            icon: "mdi:harddisk",
            value_template: "",
        })
        .with_device_class(DeviceClass::DataSize)
        .with_state_class(StateClass::Measurement)
        .with_unit_of_measurement("B");
        vec![
            SensorDiscovery {
                id: "disk_use".into(),
                title: "Disk use".into(),
                value_template: "{{ value_json.disk_use }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "disk_free".into(),
                title: "Disk free".into(),
                value_template: "{{ value_json.disk_free }}".into(),
                entity_category: Some(EntityCategory::Diagnostic),
                ..base_discovery
            },
        ]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let stat = statvfs("/").context("Failed to read statvfs")?;
        let block_size = stat.fragment_size() as u64;
        let disk_use = (stat.blocks() - stat.blocks_free()) as u64 * block_size;
        let disk_free = stat.blocks_available() as u64 * block_size;
        Ok(Payload {
            disk_use,
            disk_free,
        })
    }
}

#[derive(Serialize)]
pub struct Payload {
    disk_use: u64,
    disk_free: u64,
}
