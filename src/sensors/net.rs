use crate::ha::values::{DeviceClass, StateClass};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use anyhow::{bail, Context, Error};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use std::fs;
use std::time::{Duration, Instant};
use tokio::sync::{watch, Mutex};

const ID: &str = "net";

pub struct NetSensor {
    topic: Box<str>,
    interfaces: Vec<String>,
    rx: Mutex<watch::Receiver<Payload>>,
}

impl NetSensor {
    pub fn new(topic_base: &str) -> Result<Self, Error> {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        let mut last_obs = DeviceStatusObservation::new()?;
        let interfaces = last_obs
            .observation
            .iter()
            .filter(|s| !s.is_lo())
            .map(|s| s.interface.clone())
            .collect();
        let (tx, rx) = watch::channel(Default::default());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let obs = DeviceStatusObservation::new()?;
                let duration = obs.timestamp - last_obs.timestamp;
                let stats = obs
                    .observation
                    .iter()
                    .filter(|s| !s.is_lo())
                    .filter_map(|status| {
                        let last_status = last_obs
                            .observation
                            .iter()
                            .find(|s| s.interface == status.interface)?;
                        let calc = |f: fn(&DeviceStatus) -> u64| {
                            ((f(status) - f(last_status)) as f32 / duration.as_secs_f32()).round()
                        };
                        let bytes_in = calc(|s| s.receive_bytes);
                        let bytes_out = calc(|s| s.transmit_bytes);
                        Some((
                            status.interface.clone(),
                            InterfaceStat {
                                bytes_in,
                                bytes_out,
                            },
                        ))
                    })
                    .collect();
                tx.send(Payload(stats))
                    .context("Failed to update payload")?;
                last_obs = obs;
            }

            // This is to specify the return type so that `?` may be used above.
            #[allow(unreachable_code)]
            Ok::<(), Error>(())
        });
        Ok(NetSensor {
            topic,
            interfaces,
            rx: Mutex::new(rx),
        })
    }
}

impl Sensor for NetSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        self.interfaces
            .iter()
            .flat_map(|interface| {
                [
                    SensorDiscovery::new(SensorDiscoveryInit {
                        id: format!("net_{interface}_bytes_in"),
                        title: format!("Network {interface} throughput in"),
                        icon: "mdi:download-network",
                        value_template: format!("{{{{ value_json['{interface}'].bytes_in }}}}"),
                    })
                    .with_device_class(DeviceClass::DataRate)
                    .with_state_class(StateClass::Measurement)
                    .with_unit_of_measurement("B/s"),
                    SensorDiscovery::new(SensorDiscoveryInit {
                        id: format!("net_{interface}_bytes_out"),
                        title: format!("Network {interface} throughput out"),
                        icon: "mdi:upload-network",
                        value_template: format!("{{{{ value_json['{interface}'].bytes_out }}}}"),
                    })
                    .with_device_class(DeviceClass::DataRate)
                    .with_state_class(StateClass::Measurement)
                    .with_unit_of_measurement("B/s"),
                ]
            })
            .collect()
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let mut rx = self.rx.try_lock().context("Failed to acquire receiver")?;
        rx.changed().await.context("Failed to wait for receiver")?;
        let payload = rx.borrow_and_update().clone();
        Ok(payload)
    }
}

#[derive(Clone, Default, Serialize)]
pub struct Payload(#[serde(serialize_with = "serialize_as_map")] Vec<(String, InterfaceStat)>);

#[derive(Clone, Serialize)]
struct InterfaceStat {
    bytes_in: f32,
    bytes_out: f32,
}

fn serialize_as_map<S, K, V>(value: &[(K, V)], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize,
    V: Serialize,
{
    let mut map = serializer.serialize_map(Some(value.len()))?;
    for (k, v) in value.iter() {
        map.serialize_entry(k, v)?;
    }
    map.end()
}

struct DeviceStatusObservation {
    timestamp: Instant,
    observation: Vec<DeviceStatus>,
}

impl DeviceStatusObservation {
    fn new() -> Result<Self, Error> {
        let timestamp = Instant::now();
        let dev = fs::read_to_string("/proc/net/dev").context("Failed to read /proc/net/dev")?;
        let observation = parse_device_status(&dev).context("Failed to parse /proc/net/dev")?;
        Ok(Self {
            timestamp,
            observation,
        })
    }
}

fn parse_device_status(s: &str) -> Result<Vec<DeviceStatus>, Error> {
    s.trim()
        .lines()
        .skip(2)
        .map(|line| {
            let line = line.trim();
            let Some((interface, status)) = line.split_once(':') else {
                bail!("Expected colon for interface name");
            };
            let mut fields = status.trim().split_ascii_whitespace();
            let interface = interface.into();

            // Receive
            let receive_bytes = parse_next_field(&mut fields, "receive bytes")?;
            let receive_packets = parse_next_field(&mut fields, "receive packets")?;
            fields.next(); // errs
            fields.next(); // drops
            fields.next(); // fifo
            fields.next(); // frame
            fields.next(); // compressed
            fields.next(); // multicast

            // Transmit
            let transmit_bytes = parse_next_field(&mut fields, "transmit bytes")?;
            let transmit_packets = parse_next_field(&mut fields, "transmit packets")?;

            Ok(DeviceStatus {
                interface,
                receive_bytes,
                receive_packets,
                transmit_bytes,
                transmit_packets,
            })
        })
        .collect()
}

fn parse_next_field<'a, I>(iter: &mut I, field: &'static str) -> Result<u64, Error>
where
    I: Iterator<Item = &'a str>,
{
    iter.next()
        .with_context(|| format!("Expected field for {field}"))?
        .parse()
        .with_context(|| format!("Failed to parse field for {field}"))
}

#[derive(Debug, Eq, PartialEq)]
struct DeviceStatus {
    interface: String,
    receive_bytes: u64,
    receive_packets: u64,
    transmit_bytes: u64,
    transmit_packets: u64,
}

impl DeviceStatus {
    fn is_lo(&self) -> bool {
        self.interface == "lo"
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_device_status, serialize_as_map, DeviceStatus};
    use serde::Serialize;

    #[test]
    fn test_serialize_as_map() {
        #[derive(Serialize)]
        struct Foo(#[serde(serialize_with = "serialize_as_map")] Vec<(&'static str, u32)>);
        let foo = Foo(vec![("a", 1), ("b", 2)]);
        let result = serde_json::to_string(&foo).unwrap();
        assert_eq!(result, r#"{"a":1,"b":2}"#);
    }

    #[test]
    fn test_parse_device_status() {
        let dev = include_str!("assets/net_dev_1");
        let status = parse_device_status(dev).unwrap();
        assert_eq!(
            status,
            &[
                DeviceStatus {
                    interface: "lo".into(),
                    receive_bytes: 10_646_157,
                    receive_packets: 118_687,
                    transmit_bytes: 10_646_157,
                    transmit_packets: 118_687
                },
                DeviceStatus {
                    interface: "eth0".into(),
                    receive_bytes: 626_446_856,
                    receive_packets: 1_867_839,
                    transmit_bytes: 525_923_915,
                    transmit_packets: 2_425_108,
                }
            ]
        );
    }
}
