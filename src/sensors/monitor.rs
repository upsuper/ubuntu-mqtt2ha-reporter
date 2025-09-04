use crate::ha::values::{DeviceClass, EntityCategory};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::Error;
use serde::ser::Error as _;
use serde::{Serialize, Serializer};
use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;

const ID: &str = "monitor";

pub struct MonitorSensor {
    topic: Box<str>,
}

impl MonitorSensor {
    pub fn new(topic_base: &str) -> Self {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        MonitorSensor { topic }
    }
}

impl Sensor for MonitorSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        vec![
            SensorDiscovery::new(SensorDiscoveryInit {
                id: ID,
                title: "Updated",
                icon: "mdi:timer",
                value_template: "{{ value_json }}",
            })
            .with_entity_category(EntityCategory::Diagnostic)
            .with_device_class(DeviceClass::Timestamp),
        ]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        Ok(Payload(OffsetDateTime::now_utc()))
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct Payload(#[serde(serialize_with = "serialize")] OffsetDateTime);

pub fn serialize<S: Serializer>(
    datetime: &OffsetDateTime,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    datetime
        .format(&Iso8601::DEFAULT)
        .map_err(S::Error::custom)?
        .serialize(serializer)
}
