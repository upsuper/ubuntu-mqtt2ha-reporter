use anyhow::{Context as _, Result};

mod connections;
mod machine_id;

pub struct HostInformation {
    pub hostname: &'static str,
    pub machine_id: &'static str,
    pub connections: Vec<(&'static str, String)>,
}

impl HostInformation {
    pub fn collect() -> Result<Self> {
        let hostname = hostname::get().context("Failed to get hostname")?;
        let hostname: &str = hostname.to_string_lossy().into_owned().leak();
        let machine_id = machine_id::get().context("Failed to get machine ID")?;
        let machine_id: &str = machine_id.hyphenated().to_string().leak();
        let connections = connections::get_connections().context("Failed to get connections")?;
        Ok(Self {
            hostname,
            machine_id,
            connections,
        })
    }
}
