use self::apt::AptSensor;
use self::cpu::CpuSensor;
use self::memory::MemorySensor;
use self::monitor::MonitorSensor;
use self::reboot::RebootSensor;
use crate::sensors::disk::DiskSensor;
use crate::sensors::load::LoadSensor;
use crate::sensors::net::NetSensor;
use anyhow::{Context, Error};

mod apt;
mod cpu;
mod disk;
mod load;
mod memory;
mod monitor;
mod net;
mod reboot;

pub struct Sensors {
    pub monitor_sensor: MonitorSensor,
    pub cpu_sensor: CpuSensor,
    pub memory_sensor: MemorySensor,
    pub disk_sensor: DiskSensor,
    pub load_sensor: LoadSensor,
    pub net_sensor: NetSensor,
    pub apt_sensor: AptSensor,
    pub reboot_sensor: RebootSensor,
}

pub fn create_sensors(topic_base: &str) -> Result<Sensors, Error> {
    let monitor_sensor = MonitorSensor::new(topic_base);
    let cpu_sensor = CpuSensor::new(topic_base).context("Failed to create CPU sensor")?;
    let memory_sensor = MemorySensor::new(topic_base);
    let disk_sensor = DiskSensor::new(topic_base);
    let load_sensor = LoadSensor::new(topic_base);
    let net_sensor = NetSensor::new(topic_base).context("Failed to create network sensor")?;
    let apt_sensor = AptSensor::new(topic_base);
    let reboot_sensor = RebootSensor::new(topic_base);
    Ok(Sensors {
        monitor_sensor,
        cpu_sensor,
        memory_sensor,
        disk_sensor,
        load_sensor,
        net_sensor,
        apt_sensor,
        reboot_sensor,
    })
}
