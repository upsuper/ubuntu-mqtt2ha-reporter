use crate::ha::values::{EntityCategory, StateClass};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use crate::utils::parser::parse_next_field;
use anyhow::{Context, Error};
use serde::Serialize;
use std::fs;

const ID: &str = "load";

pub struct LoadSensor {
    topic: Box<str>,
}

impl LoadSensor {
    pub fn new(topic_base: &str) -> Self {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        LoadSensor { topic }
    }
}

impl Sensor for LoadSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        let base_discovery = SensorDiscovery::new(SensorDiscoveryInit {
            id: "",
            title: "",
            icon: "mdi:cpu-64-bit",
            value_template: "",
        })
        .with_state_class(StateClass::Measurement)
        .with_entity_category(EntityCategory::Diagnostic);
        vec![
            SensorDiscovery {
                id: "load_1min".into(),
                title: "Load (1m)".into(),
                value_template: "{{ value_json.load_1min }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "load_5min".into(),
                title: "Load (5m)".into(),
                value_template: "{{ value_json.load_5min }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "load_15min".into(),
                title: "Load (15m)".into(),
                value_template: "{{ value_json.load_15min }}".into(),
                ..base_discovery
            },
        ]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let load_avg =
            fs::read_to_string("/proc/loadavg").context("Failed to read loadavg file")?;
        let mut iter = load_avg.trim().split_ascii_whitespace();
        let load_1min = parse_next_field(&mut iter).context("1min")?;
        let load_5min = parse_next_field(&mut iter).context("5min")?;
        let load_15min = parse_next_field(&mut iter).context("15min")?;
        Ok(Payload {
            load_1min,
            load_5min,
            load_15min,
        })
    }
}

#[derive(Serialize)]
pub struct Payload {
    load_1min: f32,
    load_5min: f32,
    load_15min: f32,
}
