use crate::ha::values::{DeviceClass, EntityCategory, StateClass};
use crate::sensor::SensorDiscovery;
use bitflags::bitflags;
use serde::Serialize;

#[derive(Serialize)]
pub struct HaSensorDiscovery<'a> {
    unique_id: &'a str,
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_category: Option<EntityCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<DeviceClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_class: Option<StateClass>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_of_measurement: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggested_display_precision: Option<i32>,

    state_topic: &'a str,
    value_template: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_attributes_topic: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_attributes_template: Option<&'static str>,

    device: &'a Device<'a>,
}

impl<'a> HaSensorDiscovery<'a> {
    pub fn new(
        unique_id: &'a str,
        topic: &'a str,
        discovery: &'a SensorDiscovery,
        device: &'a Device<'a>,
    ) -> Self {
        let (attrs_topic, attrs_tpl) = match discovery.attributes_template {
            Some(tpl) => (Some(topic), Some(tpl)),
            None => (None, None),
        };
        HaSensorDiscovery {
            unique_id,
            name: &discovery.title,
            icon: discovery.icon,
            entity_category: discovery.entity_category,
            device_class: discovery.device_class,
            state_class: discovery.state_class,
            unit_of_measurement: discovery.unit_of_measurement,
            suggested_display_precision: discovery.suggested_display_precision,
            state_topic: topic,
            value_template: &discovery.value_template,
            json_attributes_topic: attrs_topic,
            json_attributes_template: attrs_tpl,
            device,
        }
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct Flags: u8 {
        const ATTRS = 1 << 0;
        const BINARY = 1 << 1;
    }
}

#[derive(Serialize)]
pub struct Device<'a> {
    pub name: Option<&'a str>,
    pub identifiers: &'a [&'a str],
}
