use anyhow::{Context, Error};
use hmac_sha256::HMAC;
use std::fs;
use uuid::{uuid, Uuid};

const MACHINE_ID_FILE: &str = "/etc/machine-id";
const APP_ID: Uuid = uuid!("860edfa6-72ea-11ef-99e1-dba127e5de37");

pub fn get() -> Result<Uuid, Error> {
    let machine_id =
        fs::read_to_string(MACHINE_ID_FILE).context("Failed to read machine-id file")?;
    let machine_id =
        Uuid::try_parse(machine_id.trim_end()).context("Failed to parse machine-id as UUID")?;
    let hmac = HMAC::mac(APP_ID, machine_id);
    let id = Uuid::from_slice(&hmac[0..16]).unwrap();
    Ok(id)
}
