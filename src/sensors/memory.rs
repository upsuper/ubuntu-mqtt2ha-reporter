use crate::ha::values::{DeviceClass, EntityCategory, StateClass};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::{ensure, Context, Error};
use serde::Serialize;
use std::fs;

const ID: &str = "memory";

pub struct MemorySensor {
    topic: Box<str>,
}

impl MemorySensor {
    pub fn new(topic_base: &str) -> Self {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        MemorySensor { topic }
    }
}

impl Sensor for MemorySensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        let base_discovery = SensorDiscovery::new(SensorDiscoveryInit {
            id: "",
            title: "",
            icon: "mdi:memory",
            value_template: "",
        })
        .with_device_class(DeviceClass::DataSize)
        .with_state_class(StateClass::Measurement)
        .with_entity_category(EntityCategory::Diagnostic)
        .with_unit_of_measurement("KiB");
        vec![
            SensorDiscovery {
                id: "memory_use".into(),
                title: "Memory use".into(),
                entity_category: None,
                value_template: "{{ value_json.mem_use }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "memory_free".into(),
                title: "Memory free".into(),
                value_template: "{{ value_json.mem_free }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "swap_use".into(),
                title: "Swap use".into(),
                value_template: "{{ value_json.swap_use }}".into(),
                ..base_discovery
            },
            SensorDiscovery {
                id: "swap_free".into(),
                title: "Swap free".into(),
                value_template: "{{ value_json.swap_free }}".into(),
                ..base_discovery
            },
        ]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let mem_info = fs::read_to_string("/proc/meminfo").context("Failed to read meminfo")?;
        let MemInfo {
            mem_total,
            mem_free,
            mem_available,
            swap_total,
            swap_free,
        } = parse_mem_info(&mem_info).context("Failed to parse mem info")?;
        // This logic is mimicking what is used in `free` command:
        // https://gitlab.com/procps-ng/procps/-/blob/2cded082b86ff6ee82174cf9f48449797695847d/library/meminfo.c#L739
        let mem_used = if mem_available > 0 && mem_available < mem_total {
            mem_total - mem_available
        } else {
            mem_total - mem_free
        };
        Ok(Payload {
            mem_use: mem_used / 1024,
            mem_free: mem_free / 1024,
            swap_use: (swap_total - swap_free) / 1024,
            swap_free: swap_free / 1024,
        })
    }
}

#[derive(Serialize)]
pub struct Payload {
    mem_use: u64,
    mem_free: u64,
    swap_use: u64,
    swap_free: u64,
}

fn parse_mem_info(mem_info: &str) -> Result<MemInfo, Error> {
    let mut mem_total = 0;
    let mut mem_free = 0;
    let mut mem_available = 0;
    let mut swap_total = 0;
    let mut swap_free = 0;
    for line in mem_info.lines() {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let slot = match label {
            "MemTotal" => &mut mem_total,
            "MemFree" => &mut mem_free,
            "MemAvailable" => &mut mem_available,
            "SwapTotal" => &mut swap_total,
            "SwapFree" => &mut swap_free,
            _ => continue,
        };
        *slot = parse_value(value).with_context(|| format!("Failed to parse {label}"))?;
    }
    Ok(MemInfo {
        mem_total,
        mem_free,
        mem_available,
        swap_total,
        swap_free,
    })
}

#[derive(Debug, Eq, PartialEq)]
struct MemInfo {
    mem_total: u64,
    mem_free: u64,
    mem_available: u64,
    swap_total: u64,
    swap_free: u64,
}

fn parse_value(s: &str) -> Result<u64, Error> {
    let s = s.trim();
    let (n, kb) = match s.split_once(' ') {
        Some((n, unit)) => {
            ensure!(unit == "kB", "Unknown unit {unit}");
            (n, true)
        }
        None => (s, false),
    };
    n.parse()
        .map(|n| if kb { n * 1024 } else { n })
        .context("Failed to parse value")
}

#[cfg(test)]
mod tests {
    use super::{parse_mem_info, MemInfo};

    #[test]
    fn test_parse_mem_info() {
        let mem_info = include_str!("assets/meminfo_1");
        assert_eq!(
            parse_mem_info(mem_info).unwrap(),
            MemInfo {
                mem_total: 65_751_344 * 1024,
                mem_free: 35_679_740 * 1024,
                mem_available: 52_700_668 * 1024,
                swap_total: 6_040_576 * 1024,
                swap_free: 6_040_576 * 1024,
            },
        );
    }
}
