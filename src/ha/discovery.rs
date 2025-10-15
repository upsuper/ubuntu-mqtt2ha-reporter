use std::borrow::Cow;

use crate::command::CommandDiscovery;
use crate::ha::values::{DeviceClass, EntityCategory, StateClass};
use crate::sensor::SensorDiscovery;
use crate::utils::serialize::serialize_as_map;
use bitflags::bitflags;
use serde::Serialize;

#[derive(Serialize)]
pub struct HaDeviceDiscovery<'a> {
    pub device: Device<'a>,
    pub origin: Origin,
    pub availability_topic: &'a str,
    #[serde(serialize_with = "serialize_as_map")]
    pub components: &'a [(String, HaComponentDiscovery<'a>)],
}

#[derive(Serialize)]
pub struct Device<'a> {
    pub name: &'static str,
    pub identifiers: &'a [&'static str],
    pub manufacturer: &'static str,
    pub model: &'static str,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    pub connections: &'a [(&'static str, String)],
}

#[derive(Serialize)]
pub struct Origin {
    pub name: &'static str,
    pub sw_version: &'static str,
}

#[derive(Serialize)]
#[serde(tag = "platform", rename_all = "snake_case")]
pub enum HaComponentDiscovery<'a> {
    Sensor(HaSensorDiscovery<'a>),
    BinarySensor(HaSensorDiscovery<'a>),
    Button(HaButtonDiscovery<'a>),
}

#[derive(Serialize)]
pub struct HaSensorDiscovery<'a> {
    unique_id: String,
    name: Cow<'a, str>,
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
    value_template: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_attributes_topic: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_attributes_template: Option<&'static str>,
}

impl<'a> HaSensorDiscovery<'a> {
    pub fn new(
        unique_id: String,
        topic: &'a str,
        discovery: SensorDiscovery<'a>,
    ) -> (Cow<'a, str>, Self) {
        let (attrs_topic, attrs_tpl) = match discovery.attributes_template {
            Some(tpl) => (Some(topic), Some(tpl)),
            None => (None, None),
        };
        (
            discovery.id,
            HaSensorDiscovery {
                unique_id,
                name: discovery.title,
                icon: discovery.icon,
                entity_category: discovery.entity_category,
                device_class: discovery.device_class,
                state_class: discovery.state_class,
                unit_of_measurement: discovery.unit_of_measurement,
                suggested_display_precision: discovery.suggested_display_precision,
                state_topic: topic,
                value_template: discovery.value_template,
                json_attributes_topic: attrs_topic,
                json_attributes_template: attrs_tpl,
            },
        )
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
pub struct HaButtonDiscovery<'a> {
    unique_id: String,
    name: Cow<'a, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entity_category: Option<EntityCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<&'static str>,
    command_topic: &'a str,
}

impl<'a> HaButtonDiscovery<'a> {
    pub fn new(
        unique_id: String,
        command_topic: &'a str,
        discovery: CommandDiscovery<'a>,
    ) -> (Cow<'a, str>, Self) {
        (
            discovery.id,
            HaButtonDiscovery {
                unique_id,
                name: discovery.name,
                icon: discovery.icon,
                entity_category: discovery.entity_category,
                device_class: discovery.device_class,
                command_topic,
            },
        )
    }
}
