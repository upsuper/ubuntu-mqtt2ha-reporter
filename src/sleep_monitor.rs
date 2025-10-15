use anyhow::{Context, Result};
use futures_util::Stream;
use futures_util::stream::StreamExt;
use log::warn;
use std::os::fd::OwnedFd;
use zbus::message::Type as MessageType;
use zbus::names::{InterfaceName, MemberName};
use zbus::{Connection, MatchRule, MessageStream, Proxy, zvariant};

const LOGIN1: &str = "org.freedesktop.login1";
const LOGIN1_PATH: &str = "/org/freedesktop/login1";
const LOGIN1_MANAGER: &str = "org.freedesktop.login1.Manager";
const PREPARE_FOR_SLEEP: &str = "PrepareForSleep";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepEvent {
    PreparingSleep,
    WakingUp,
}

pub struct SleepMonitor {
    connection: Connection,
}

impl SleepMonitor {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system()
            .await
            .context("Failed to connect to system D-Bus")?;
        Ok(Self { connection })
    }

    pub async fn take_inhibitor_lock(&self) -> Result<InhibitorLock> {
        InhibitorLock::new(&self.connection).await
    }

    pub async fn start_monitoring(&self) -> Result<impl Stream<Item = Result<SleepEvent>>> {
        let match_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .interface(LOGIN1_MANAGER)
            .unwrap()
            .member(PREPARE_FOR_SLEEP)
            .unwrap()
            .build();
        let stream = MessageStream::for_match_rule(match_rule, &self.connection, None)
            .await
            .context("Failed to create message stream for PrepareForSleep signal")?;

        let stream = stream.filter_map(async |msg| {
            let msg = match msg {
                Ok(msg) => msg,
                Err(e) => return Some(Err(e).context("Failed to receive message")),
            };
            let header = msg.header();
            if header.interface().map(InterfaceName::as_str) != Some(LOGIN1_MANAGER)
                || header.member().map(MemberName::as_str) != Some(PREPARE_FOR_SLEEP)
            {
                return None;
            }
            let body = msg.body();
            let Ok(preparing_for_sleep) = body.deserialize::<bool>() else {
                warn!("Failed to deserialize PrepareForSleep signal body");
                return None;
            };
            Some(Ok(if preparing_for_sleep {
                SleepEvent::PreparingSleep
            } else {
                SleepEvent::WakingUp
            }))
        });
        Ok(stream)
    }
}

pub struct InhibitorLock {
    // The file descriptor is kept for releasing the lock on drop.
    _fd: OwnedFd,
}

impl InhibitorLock {
    async fn new(connection: &Connection) -> Result<Self> {
        let proxy = Proxy::new(connection, LOGIN1, LOGIN1_PATH, LOGIN1_MANAGER)
            .await
            .context("Failed to create login1 manager proxy")?;
        let reply = proxy
            .call_method(
                "Inhibit",
                &(
                    "sleep",
                    "ubuntu-mqtt2ha-reporter",
                    "Need to report unavailability before sleep",
                    "delay",
                ),
            )
            .await
            .context("Failed to call Inhibit method")?;
        let fd = reply
            .body()
            .deserialize::<zvariant::OwnedFd>()
            .context("Failed to deserialize inhibitor file descriptor")?;
        Ok(InhibitorLock { _fd: fd.into() })
    }
}
