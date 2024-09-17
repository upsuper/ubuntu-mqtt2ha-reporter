use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub daemon: Daemon,
    pub mqtt: Mqtt,
}

#[derive(Debug, Deserialize)]
pub struct Daemon {
    /// Fixed interval in minutes to report status to broker. (Default: 5)
    #[serde(default = "Daemon::default_interval_in_minutes")]
    pub interval_in_minutes: u16,
}

impl Daemon {
    fn default_interval_in_minutes() -> u16 {
        5
    }
}

#[derive(Debug, Deserialize)]
pub struct Mqtt {
    /// The hostname or IP address of the MQTT broker to connect to. (Default: localhost)
    #[serde(default = "Mqtt::default_hostname")]
    pub hostname: String,
    /// The TCP port the MQTT broker is listening on. (Default: 1883)
    #[serde(default = "Mqtt::default_port")]
    pub port: u16,
    /// Maximum period in seconds between ping messages to the broker. (Default: 60)
    #[serde(default = "Mqtt::default_keep_alive")]
    pub keep_alive: u64,
    /// Enable TLS/SSL on the connection. (Default: false)
    #[serde(default = "Mqtt::default_tls")]
    pub tls: bool,
    /// Path to CA certificate file to verify host.
    pub tls_ca_cert: Option<PathBuf>,

    /// The MQTT broker authentication credentials. (Default: no authentication)
    pub username: Option<String>,
    /// The MQTT broker authentication credentials. (Default: no authentication)
    pub password: Option<String>,

    /// By default, Home Assistant listens to the `/homeassistant`,
    /// but it can be changed for a given installation.
    /// Likewise, by default this application advertises the same default topic.
    /// If you use a different discovery prefix, then specify yours here.
    /// (Default: homeassistant)
    #[serde(default = "Mqtt::default_discovery_prefix")]
    pub discovery_prefix: String,
    /// The MQTT base topic under which to publish the sensor data topics. (Default: home/nodes)
    ///
    /// Note: the MQTT topic used for this device is constructed as `{base_topic}/{sensor_name}`.
    #[serde(default = "Mqtt::default_base_topic")]
    pub base_topic: String,
}

impl Mqtt {
    fn default_hostname() -> String {
        "localhost".to_owned()
    }
    fn default_port() -> u16 {
        1883
    }
    fn default_keep_alive() -> u64 {
        60
    }
    fn default_tls() -> bool {
        false
    }
    fn default_discovery_prefix() -> String {
        "homeassistant".to_owned()
    }
    fn default_base_topic() -> String {
        "home/nodes".to_owned()
    }
}
