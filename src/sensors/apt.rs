use crate::ha::values::StateClass;
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::{ensure, Context, Error};
use regex::Regex;
use serde::Serialize;
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const ID: &str = "apt";

pub struct AptSensor {
    topic: Box<str>,
}

impl AptSensor {
    pub fn new(topic_base: &str) -> Self {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        AptSensor { topic }
    }
}

impl Sensor for AptSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        vec![SensorDiscovery::new(SensorDiscoveryInit {
            id: ID,
            title: "APT pending upgrades",
            icon: "mdi:update",
            value_template: "{{ value_json.state }}",
        })
        .with_state_class(StateClass::Measurement)
        .with_attributes("{{ value_json.attrs | tojson }}")]
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let mut child = Command::new("apt-get")
            .args(["--just-print", "upgrade"])
            .env("LANG", "C")
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to invoke apt-get")?;

        let stdout = child.stdout.take().context("Failed to take stdout")?;
        let handle = tokio::spawn(async move {
            let mut list = Vec::new();
            let mut reader = BufReader::new(stdout).lines();
            while let Some(line) = reader.next_line().await? {
                if let Some(captures) = REGEX_INST.captures(&line) {
                    list.push(captures.get(1).unwrap().as_str().to_owned());
                }
            }
            Ok::<_, Error>(list)
        });

        let status = child
            .wait()
            .await
            .context("Failed to wait for apt-get command")?;
        ensure!(status.success(), "apt-get command failed");

        let list = handle
            .await
            .context("Failed to wait for apt-get output parsing")?
            .context("Failed to parse apt-get output")?;

        let state = list.len();
        let attrs = Attrs { packages: list };
        Ok(Payload { state, attrs })
    }
}

#[derive(Serialize)]
pub struct Payload {
    state: usize,
    attrs: Attrs,
}

#[derive(Serialize)]
struct Attrs {
    packages: Vec<String>,
}

static REGEX_INST: LazyLock<Regex> = LazyLock::new(|| Regex::new("^Inst ([^ ]+)").unwrap());
