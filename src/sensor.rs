use crate::ha::values::{DeviceClass, EntityCategory, StateClass};
use anyhow::Error;
use serde::Serialize;
use std::borrow::Cow;

pub trait Sensor: 'static {
    type Payload: Serialize + 'static;

    fn topic(&self) -> &str;

    fn discovery_data(&self) -> Vec<SensorDiscovery<'_>>;

    async fn get_status(&self) -> Result<Self::Payload, Error>;
}

pub struct SensorDiscovery<'a> {
    pub id: Cow<'a, str>,
    pub title: Cow<'a, str>,
    pub entity_category: Option<EntityCategory>,
    pub device_class: Option<DeviceClass>,
    pub state_class: Option<StateClass>,
    pub unit_of_measurement: Option<&'static str>,
    pub suggested_display_precision: Option<i32>,
    pub icon: Option<&'static str>,
    pub binary: bool,
    pub value_template: Cow<'a, str>,
    pub attributes_template: Option<&'static str>,
}

impl<'a> SensorDiscovery<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(init: SensorDiscoveryInit<S>) -> Self {
        Self {
            id: init.id.into(),
            title: init.title.into(),
            entity_category: None,
            device_class: None,
            state_class: None,
            unit_of_measurement: None,
            suggested_display_precision: None,
            icon: Some(init.icon),
            binary: false,
            value_template: init.value_template.into(),
            attributes_template: None,
        }
    }

    pub fn with_entity_category(mut self, entity_category: EntityCategory) -> Self {
        self.entity_category = Some(entity_category);
        self
    }

    pub fn with_device_class(mut self, device_class: DeviceClass) -> Self {
        self.device_class = Some(device_class);
        self
    }

    pub fn with_state_class(mut self, state_class: StateClass) -> Self {
        self.state_class = Some(state_class);
        self
    }

    pub fn with_unit_of_measurement(mut self, unit_of_measurement: &'static str) -> Self {
        self.unit_of_measurement = Some(unit_of_measurement);
        self
    }

    pub fn with_suggested_display_precision(mut self, suggested_display_precision: i32) -> Self {
        self.suggested_display_precision = Some(suggested_display_precision);
        self
    }

    pub fn with_binary(mut self, binary: bool) -> Self {
        self.binary = binary;
        self
    }

    pub fn with_attributes(mut self, template: &'static str) -> Self {
        self.attributes_template = Some(template);
        self
    }
}

pub struct SensorDiscoveryInit<S> {
    pub id: S,
    pub title: S,
    pub icon: &'static str,
    pub value_template: S,
}
