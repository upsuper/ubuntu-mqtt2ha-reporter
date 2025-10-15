use anyhow::Error;
use async_trait::async_trait;
use serde::Serialize;
use std::borrow::Cow;
use crate::ha::values::EntityCategory;

#[async_trait]
pub trait Command: 'static {
    fn topic(&self) -> &str;
    fn discovery_data(&self) -> Vec<CommandDiscovery<'_>>;
    async fn execute(&self) -> Result<(), Error>;
}

#[derive(Serialize)]
pub struct CommandDiscovery<'a> {
    pub id: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub icon: Option<&'static str>,
    pub entity_category: Option<EntityCategory>,
    pub device_class: Option<&'static str>,
}

impl<'a> CommandDiscovery<'a> {
    pub fn new<S: Into<Cow<'a, str>>>(init: CommandDiscoveryInit<S>) -> Self {
        Self {
            id: init.id.into(),
            name: init.name.into(),
            icon: Some(init.icon),
            entity_category: None,
            device_class: None,
        }
    }

    pub fn with_entity_category(mut self, entity_category: EntityCategory) -> Self {
        self.entity_category = Some(entity_category);
        self
    }

    pub fn with_device_class(mut self, device_class: &'static str) -> Self {
        self.device_class = Some(device_class);
        self
    }
}

pub struct CommandDiscoveryInit<S> {
    pub id: S,
    pub name: S,
    pub icon: &'static str,
}
