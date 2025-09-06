use crate::command::Command;
use crate::commands::Commands;
use crate::ha::discovery::{
    Device, HaButtonDiscovery, HaComponentDiscovery, HaDeviceDiscovery, HaSensorDiscovery, Origin,
};
use crate::sensor::Sensor;
use crate::sensors::Sensors;
use crate::utils::snake_case::make_snake_case;
use anyhow::{Context as _, Error};
use log::debug;
use rumqttc::{AsyncClient, QoS};
use std::borrow::Cow;

pub async fn publish_discovery(
    client: &AsyncClient,
    availability_topic: &str,
    discovery_prefix: &str,
    hostname: &'static str,
    machine_id: &'static str,
    connections: &[(&'static str, String)],
    sensors: &Sensors,
    commands: &Commands,
) -> Result<(), Error> {
    let Sensors {
        monitor_sensor,
        cpu_sensor,
        memory_sensor,
        disk_sensor,
        load_sensor,
        net_sensor,
        apt_sensor,
        reboot_sensor,
    } = sensors;
    let Commands {
        reboot_command,
        suspend_command,
    } = commands;

    let hostname_snake = make_snake_case(hostname);
    let discovery_topic = format!("{discovery_prefix}/device/{hostname_snake}/config");

    // Construct payload of discovery message
    let device = Device {
        name: hostname,
        identifiers: &[machine_id],
        connections,
    };
    let origin = Origin {
        name: env!("CARGO_PKG_NAME"),
        sw_version: env!("CARGO_PKG_VERSION"),
    };
    let components = {
        let mut collector = ComponentCollector::new(&hostname_snake);
        collector.add_sensor(monitor_sensor);
        collector.add_sensor(cpu_sensor);
        collector.add_sensor(memory_sensor);
        collector.add_sensor(disk_sensor);
        collector.add_sensor(load_sensor);
        collector.add_sensor(net_sensor);
        collector.add_sensor(apt_sensor);
        collector.add_sensor(reboot_sensor);
        collector.add_command(reboot_command);
        collector.add_command(suspend_command);
        collector.result
    };
    let discovery = HaDeviceDiscovery {
        device,
        origin,
        availability_topic,
        components: &components,
    };
    let payload = serde_json::to_string(&discovery).unwrap();

    debug!("Publishing {} to {}", payload, discovery_topic);
    client
        .publish(discovery_topic, QoS::AtLeastOnce, true, payload)
        .await
        .context("Failed to publish discovery")?;

    Ok(())
}

struct ComponentCollector<'a> {
    hostname_snake: &'a str,
    result: Vec<(Cow<'a, str>, HaComponentDiscovery<'a>)>,
}

impl<'a> ComponentCollector<'a> {
    fn new(hostname_snake: &'a str) -> Self {
        Self {
            hostname_snake,
            result: Vec::new(),
        }
    }

    fn add_sensor<S: Sensor>(&mut self, sensor: &'a S) {
        self.result
            .extend(sensor.discovery_data().into_iter().map(|item| {
                let sensor_id = format!("{}_{}", self.hostname_snake, item.id);
                let (id, discovery) = HaSensorDiscovery::new(sensor_id, sensor.topic(), item);
                (id, HaComponentDiscovery::Sensor(discovery))
            }));
    }

    fn add_command<C: Command>(&mut self, command: &'a C) {
        self.result
            .extend(command.discovery_data().into_iter().map(|item| {
                let command_id = format!("{}_{}", self.hostname_snake, item.id);
                let (id, discovery) = HaButtonDiscovery::new(command_id, command.topic(), item);
                (id, HaComponentDiscovery::Button(discovery))
            }));
    }
}
