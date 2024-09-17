use crate::ha::values::{EntityCategory, StateClass};
use crate::sensor::{Sensor, SensorDiscovery, SensorDiscoveryInit};
use crate::utils::parser::{parse_next_field, parse_next_field_opt};
use anyhow::{Context, Error};
use nix::unistd::{sysconf, SysconfVar};
use serde::Serialize;
use std::fs;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::sync::{watch, Mutex};

const ID: &str = "cpu";

pub struct CpuSensor {
    topic: Box<str>,
    cpu_count: usize,
    rx: Mutex<watch::Receiver<Payload>>,
}

impl CpuSensor {
    pub fn new(topic_base: &str) -> Result<Self, Error> {
        let topic = format!("{topic_base}/{ID}").into_boxed_str();
        let mut last_obs = CpuTimesObservation::new()?;
        let cpu_count = last_obs.observation.per_cpu.len();
        let clock_tick = sysconf(SysconfVar::CLK_TCK)
            .context("Failed to read clock tick")?
            .context("Expected clock tick to be available")? as u64;
        let (tx, rx) = watch::channel(Default::default());
        tokio::spawn(async move {
            loop {
                // Calculate average CPU use over 1 minute.
                tokio::time::sleep(Duration::from_secs(60)).await;
                let obs = CpuTimesObservation::new()?;
                let duration = obs.timestamp - last_obs.timestamp;
                let total = round_percentage(
                    calculate_percentage(
                        &last_obs.observation.total,
                        &obs.observation.total,
                        clock_tick,
                        duration,
                    ) / cpu_count as f32,
                );
                let per_cpu = if cpu_count > 1 {
                    Iterator::zip(
                        last_obs.observation.per_cpu.iter(),
                        obs.observation.per_cpu.iter(),
                    )
                    .map(|(start, end)| {
                        round_percentage(calculate_percentage(start, end, clock_tick, duration))
                    })
                    .collect()
                } else {
                    Vec::new()
                };
                tx.send(Payload { total, per_cpu })
                    .context("Failed to update payload")?;
                last_obs = obs;
            }

            // This is to specify the return type so that `?` may be used above.
            #[allow(unreachable_code)]
            Ok::<(), Error>(())
        });
        Ok(CpuSensor {
            topic,
            cpu_count,
            rx: Mutex::new(rx),
        })
    }
}

impl Sensor for CpuSensor {
    type Payload = Payload;

    fn topic(&self) -> &str {
        self.topic.as_ref()
    }

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>> {
        let mut data = Vec::with_capacity(self.cpu_count + 1);
        let base_discovery = SensorDiscovery::new(SensorDiscoveryInit {
            id: "",
            title: "",
            icon: "mdi:cpu-64-bit",
            value_template: "",
        })
        .with_entity_category(EntityCategory::Diagnostic)
        .with_state_class(StateClass::Measurement)
        .with_unit_of_measurement("%")
        .with_suggested_display_precision(1);
        data.push(SensorDiscovery {
            id: ID.into(),
            title: "CPU use".into(),
            value_template: "{{ value_json.total }}".into(),
            entity_category: None,
            ..base_discovery
        });
        if self.cpu_count > 1 {
            for i in 0..self.cpu_count {
                data.push(SensorDiscovery {
                    id: format!("cpu_{i}").into(),
                    title: format!("CPU {i} use").into(),
                    value_template: format!("{{{{ value_json.per_cpu[{i}] }}}}").into(),
                    ..base_discovery
                });
            }
        }
        data
    }

    async fn get_status(&self) -> Result<Self::Payload, Error> {
        let mut rx = self.rx.try_lock().context("Failed to acquire receiver")?;
        rx.changed().await.context("Failed to wait for receiver")?;
        let payload = rx.borrow_and_update().clone();
        Ok(payload)
    }
}

#[derive(Clone, Default, Serialize)]
pub struct Payload {
    total: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    per_cpu: Vec<f32>,
}

struct CpuTimesObservation {
    timestamp: Instant,
    observation: AllCpuTimes,
}

impl CpuTimesObservation {
    fn new() -> Result<Self, Error> {
        let timestamp = Instant::now();
        let stat = fs::read_to_string("/proc/stat").context("Failed to read stat file")?;
        let observation = parse_stat(&stat).context("Failed to parse stat file")?;
        Ok(Self {
            timestamp,
            observation,
        })
    }
}

fn parse_stat(s: &str) -> Result<AllCpuTimes, Error> {
    let mut lines = s.lines();
    let first_line = lines
        .next()
        .context("Expected total CPU times")?
        .strip_prefix("cpu ")
        .context("Expected line starting with cpu")?
        .trim();
    let total = CpuTimes::from_str(first_line).context("For parsing total CPU times")?;

    let mut per_cpu = Vec::new();
    for (i, line) in lines.enumerate() {
        let Some(line) = line.strip_prefix("cpu") else {
            break;
        };
        let (_, s) = line
            .split_once(' ')
            .context("Unexpected CPU times format")?;
        let cpu_times =
            CpuTimes::from_str(s).with_context(|| format!("For parsing times of CPU {i}"))?;
        per_cpu.push(cpu_times);
    }

    Ok(AllCpuTimes { total, per_cpu })
}

#[derive(Debug, Eq, PartialEq)]
struct AllCpuTimes {
    total: CpuTimes,
    per_cpu: Vec<CpuTimes>,
}

#[derive(Debug, Eq, PartialEq)]
struct CpuTimes {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    io_wait: u64,
    irq: u64,
    soft_irq: u64,
    steal: u64,
    guest: u64,
    guest_nice: u64,
}

impl CpuTimes {
    fn busy(&self) -> u64 {
        // XXX: Is this the right way to count busy time?
        self.user + self.nice + self.system + self.guest + self.guest_nice
    }
}

impl FromStr for CpuTimes {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = s.split_ascii_whitespace();
        let user = parse_next_field(&mut fields).context("user")?;
        let nice = parse_next_field(&mut fields).context("nice")?;
        let system = parse_next_field(&mut fields).context("system")?;
        let idle = parse_next_field(&mut fields).context("idle")?;
        let io_wait = parse_next_field_opt(&mut fields).context("iowait")?;
        let irq = parse_next_field_opt(&mut fields).context("irq")?;
        let soft_irq = parse_next_field_opt(&mut fields).context("softirq")?;
        let steal = parse_next_field_opt(&mut fields).context("steal")?;
        let guest = parse_next_field_opt(&mut fields).context("guest")?;
        let guest_nice = parse_next_field_opt(&mut fields).context("guest_nice")?;
        Ok(CpuTimes {
            user,
            nice,
            system,
            idle,
            io_wait,
            irq,
            soft_irq,
            steal,
            guest,
            guest_nice,
        })
    }
}

fn calculate_percentage(
    start: &CpuTimes,
    end: &CpuTimes,
    clock_tick: u64,
    duration: Duration,
) -> f32 {
    let start = start.busy();
    let end = end.busy();
    let delta = (end - start) as f32 / clock_tick as f32;
    delta / duration.as_secs_f32() * 100.
}

fn round_percentage(p: f32) -> f32 {
    (p * 10.).round() / 10.
}

#[cfg(test)]
mod tests {
    use super::{parse_stat, AllCpuTimes, CpuTimes};

    #[test]
    fn test_stat() {
        let stat = include_str!("assets/stat_1");
        let all_cpu_times = parse_stat(stat).unwrap();
        assert_eq!(
            all_cpu_times,
            AllCpuTimes {
                total: CpuTimes {
                    user: 307063,
                    nice: 258,
                    system: 146959,
                    idle: 9876550,
                    io_wait: 5476,
                    irq: 0,
                    soft_irq: 1753,
                    steal: 0,
                    guest: 0,
                    guest_nice: 0,
                },
                per_cpu: vec![
                    CpuTimes {
                        user: 12429,
                        nice: 5,
                        system: 6413,
                        idle: 408107,
                        io_wait: 254,
                        irq: 0,
                        soft_irq: 515,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13952,
                        nice: 70,
                        system: 6075,
                        idle: 409951,
                        io_wait: 287,
                        irq: 0,
                        soft_irq: 242,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13986,
                        nice: 4,
                        system: 6310,
                        idle: 410325,
                        io_wait: 236,
                        irq: 0,
                        soft_irq: 123,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 16947,
                        nice: 10,
                        system: 7479,
                        idle: 405690,
                        io_wait: 267,
                        irq: 0,
                        soft_irq: 285,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 16288,
                        nice: 50,
                        system: 7191,
                        idle: 407055,
                        io_wait: 279,
                        irq: 0,
                        soft_irq: 30,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 15110,
                        nice: 9,
                        system: 6560,
                        idle: 409107,
                        io_wait: 261,
                        irq: 0,
                        soft_irq: 32,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13662,
                        nice: 6,
                        system: 6077,
                        idle: 411090,
                        io_wait: 214,
                        irq: 0,
                        soft_irq: 36,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 12496,
                        nice: 6,
                        system: 5780,
                        idle: 412657,
                        io_wait: 203,
                        irq: 0,
                        soft_irq: 37,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 12528,
                        nice: 4,
                        system: 5672,
                        idle: 412701,
                        io_wait: 192,
                        irq: 0,
                        soft_irq: 39,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13420,
                        nice: 3,
                        system: 5890,
                        idle: 411182,
                        io_wait: 218,
                        irq: 0,
                        soft_irq: 77,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13757,
                        nice: 3,
                        system: 6204,
                        idle: 410682,
                        io_wait: 263,
                        irq: 0,
                        soft_irq: 31,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13697,
                        nice: 27,
                        system: 6373,
                        idle: 410555,
                        io_wait: 272,
                        irq: 0,
                        soft_irq: 35,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 11732,
                        nice: 3,
                        system: 5174,
                        idle: 413987,
                        io_wait: 187,
                        irq: 0,
                        soft_irq: 45,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 9604,
                        nice: 4,
                        system: 4516,
                        idle: 416547,
                        io_wait: 159,
                        irq: 0,
                        soft_irq: 203,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 9564,
                        nice: 3,
                        system: 4720,
                        idle: 416742,
                        io_wait: 219,
                        irq: 0,
                        soft_irq: 2,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 11346,
                        nice: 4,
                        system: 5618,
                        idle: 413823,
                        io_wait: 208,
                        irq: 0,
                        soft_irq: 2,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 10693,
                        nice: 3,
                        system: 10574,
                        idle: 406462,
                        io_wait: 227,
                        irq: 0,
                        soft_irq: 2,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 12458,
                        nice: 4,
                        system: 6343,
                        idle: 411879,
                        io_wait: 224,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 11224,
                        nice: 8,
                        system: 5091,
                        idle: 414703,
                        io_wait: 212,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 11617,
                        nice: 2,
                        system: 5602,
                        idle: 413599,
                        io_wait: 210,
                        irq: 0,
                        soft_irq: 2,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 11687,
                        nice: 7,
                        system: 5363,
                        idle: 414016,
                        io_wait: 195,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13141,
                        nice: 5,
                        system: 6446,
                        idle: 411081,
                        io_wait: 201,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 13064,
                        nice: 6,
                        system: 5785,
                        idle: 412003,
                        io_wait: 267,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                    CpuTimes {
                        user: 12649,
                        nice: 5,
                        system: 5689,
                        idle: 412597,
                        io_wait: 210,
                        irq: 0,
                        soft_irq: 1,
                        steal: 0,
                        guest: 0,
                        guest_nice: 0
                    },
                ],
            }
        )
    }
}
