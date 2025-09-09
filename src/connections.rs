use anyhow::{Context as _, Result};
use serde::Deserialize;
use std::fmt;
use std::process::Command;

pub fn get_connections() -> Result<Vec<(&'static str, String)>> {
    let mut connections = Vec::new();
    connections.extend(get_mac_addresses()?.into_iter().map(|addr| ("mac", addr)));
    Ok(connections)
}

fn get_mac_addresses() -> Result<Vec<String>> {
    let output = Command::new("ip")
        .args(["-j", "addr", "show"])
        .output()
        .context("Failed to execute 'ip addr show'")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "ip command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let json_str =
        String::from_utf8(output.stdout).context("Failed to parse ip command output as UTF-8")?;
    let interfaces: Vec<Interface> =
        serde_json::from_str(&json_str).context("Failed to parse ip command output as JSON")?;
    let addresses = interfaces
        .into_iter()
        .filter_map(|interface| {
            let Some(address) = interface.address else {
                return None;
            };
            (interface.operate_state == OperateState::Up
                && interface
                    .addr_info
                    .iter()
                    .any(|addr| addr.scope == Scope::Global))
                .then_some(address)
        })
        .collect();
    Ok(addresses)
}

#[derive(Deserialize)]
struct Interface {
    #[serde(rename = "operstate")]
    operate_state: OperateState,
    address: Option<String>,
    addr_info: Vec<AddrInfo>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum OperateState {
    Unknown,
    Up,
    Down,
}

#[derive(Deserialize)]
struct AddrInfo {
    scope: Scope,
}

#[derive(Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Scope {
    Host,
    Link,
    Global,
}

pub struct Display<'a>(pub &'a [(&'static str, String)]);

impl<'a> fmt::Display for Display<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (key, value) in self.0 {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            write!(f, "{}={}", key, value)?;
        }
        Ok(())
    }
}
