use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::{Context, Error};
use futures_util::TryStreamExt;
use log::warn;
use serde::Serialize;
use std::io;
use tokio::fs;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_stream::wrappers::LinesStream;

const ID: &str = "reboot";

pub struct RebootSensor {
    topic: Box<str>,
}

impl RebootSensor {
    pub fn new(topic_base: &str) -> RebootSensor {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        RebootSensor { topic }
    }
}

impl Sensor for RebootSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        vec![SensorDiscovery::new(SensorDiscoveryInit {
            id: ID,
            title: "Reboot required",
            icon: "mdi:restart",
            value_template: "{{ 'ON' if value_json.state else 'OFF' }}",
        })
        .with_binary(true)
        .with_attributes("{{ value_json.attrs | tojson }}")]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let state = fs::try_exists("/var/run/reboot-required")
            .await
            .context("Failed to check existence of reboot-required file")?;
        let packages = if state {
            read_packages().await.unwrap_or_else(|e| {
                warn!("Failed to read packages: {e}");
                vec![]
            })
        } else {
            vec![]
        };
        let attrs = Attrs { packages };
        Ok(Payload { state, attrs })
    }
}

async fn read_packages() -> Result<Vec<String>, Error> {
    let file = match File::open("/var/run/reboot-required.pkgs").await {
        Ok(file) => file,
        Err(e) => {
            if e.kind() == io::ErrorKind::NotFound {
                return Ok(Vec::new());
            }
            return Err(e).context("Failed to open reboot-required.pkgs");
        }
    };
    LinesStream::new(BufReader::new(file).lines())
        .try_collect()
        .await
        .context("Failed to read reboot-required.pkgs")
}

#[derive(Serialize)]
pub struct Payload {
    state: bool,
    attrs: Attrs,
}

#[derive(Serialize)]
struct Attrs {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    packages: Vec<String>,
}
