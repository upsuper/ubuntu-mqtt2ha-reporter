use serde::Serialize;

/// Device class is a measurement categorization in Home Assistant.
///
/// See https://www.home-assistant.io/integrations/sensor/#device-class.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceClass {
    DataRate,
    DataSize,
    Timestamp,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateClass {
    Measurement,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityCategory {
    Config,
    Diagnostic,
}
