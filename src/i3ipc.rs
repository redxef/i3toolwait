use anyhow::Result;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream};
use tokio::net::UnixStream;

pub async fn get_socket_path() -> Result<std::path::PathBuf, anyhow::Error> {
    if let Ok(p) = std::env::var("I3SOCK") {
        return Ok(std::path::PathBuf::from_str(&p).unwrap());
    }
    if let Ok(p) = std::env::var("SWAYSOCK") {
        return Ok(std::path::PathBuf::from_str(&p).unwrap());
    }

    for command_name in ["i3", "sway"] {
        let output = tokio::process::Command::new(command_name)
            .arg("--get-socketpath")
            .output()
            .await?;
        if output.status.success() {
            return Ok(std::path::PathBuf::from_str(
                String::from_utf8_lossy(&output.stdout).trim_end_matches('\n'),
            )
            .unwrap());
        }
    }

    Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, ""))?
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum MessageType {
    Command = 0,
    Workspace = 1,
    Subscribe = 2,
    Outputs = 3,
    Tree = 4,
    Marks = 5,
    BarConfig = 6,
    Version = 7,
    BindingModes = 8,
    Config = 9,
    Tick = 10,
    Sync = 11,
    BindingState = 12,
    #[serde(rename = "workspace")]
    SubWorkspace = 0 | 1 << 31,
    #[serde(rename = "output")]
    SubOutput = 1 | 1 << 31,
    #[serde(rename = "mode")]
    SubMode = 2 | 1 << 31,
    #[serde(rename = "window")]
    SubWindow = 3 | 1 << 31,
    #[serde(rename = "barconfig_update")]
    SubBarConfig = 4 | 1 << 31,
    #[serde(rename = "binding")]
    SubBinding = 5 | 1 << 31,
    #[serde(rename = "shutdown")]
    SubShutdown = 6 | 1 << 31,
    #[serde(rename = "tick")]
    SubTick = 7 | 1 << 31,
}

impl MessageType {
    pub fn is_subscription(&self) -> bool {
        ((*self as u32) & (1 << 31)) != 0
    }
}

impl TryFrom<u32> for MessageType {
    type Error = &'static str;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00000000 => Self::Command,
            0x00000001 => Self::Workspace,
            0x00000002 => Self::Subscribe,
            0x00000003 => Self::Outputs,
            0x00000004 => Self::Tree,
            0x00000005 => Self::Marks,
            0x00000006 => Self::BarConfig,
            0x00000007 => Self::Version,
            0x00000008 => Self::BindingModes,
            0x00000009 => Self::Config,
            0x0000000a => Self::Tick,
            0x0000000b => Self::Sync,
            0x0000000c => Self::BindingState,
            0x80000000 => Self::SubWorkspace,
            0x80000001 => Self::SubOutput,
            0x80000002 => Self::SubMode,
            0x80000003 => Self::SubWindow,
            0x80000004 => Self::SubBarConfig,
            0x80000005 => Self::SubBinding,
            0x80000006 => Self::SubShutdown,
            0x80000007 => Self::SubTick,
            _ => return Err(""),
        })
    }
}

type SubscriptionCallback =
    dyn Fn(
        MessageType,
        serde_json::Value,
    ) -> Pin<Box<dyn std::future::Future<Output = Vec<(MessageType, Vec<u8>)>> + Send>>;

pub struct Connection<'a> {
    stream: BufStream<UnixStream>,
    subscriptions: HashMap<MessageType, Box<&'a SubscriptionCallback>>,
}

impl<'a> Connection<'a> {
    pub fn connect(path: &std::path::Path) -> Result<Self, anyhow::Error> {
        let stream = std::os::unix::net::UnixStream::connect(path)?;
        stream.set_nonblocking(true)?;
        let stream = BufStream::new(UnixStream::from_std(stream)?);
        let subscriptions = HashMap::new();
        Ok(Self {
            stream,
            subscriptions,
        })
    }

    pub async fn send_message(
        &mut self,
        message_type: &MessageType,
        message: &[u8],
    ) -> Result<(), anyhow::Error> {
        self.stream.write_all(b"i3-ipc").await?;
        self.stream.write_u32_le(message.len() as u32).await?;
        self.stream.write_u32_le(*message_type as u32).await?;
        self.stream.write_all(message).await?;
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn receive_message(&mut self) -> Result<(MessageType, Vec<u8>), anyhow::Error> {
        let mut buffer = vec![0u8; 6];
        self.stream.read_exact(&mut buffer).await?;
        if buffer != b"i3-ipc" {
            return Err(tokio::io::Error::new(tokio::io::ErrorKind::Other, ""))?;
        }
        let message_len = self.stream.read_u32_le().await?;
        let message_type = self.stream.read_u32_le().await?.try_into().unwrap();
        let mut buffer = vec![0u8; message_len as usize];
        self.stream.read_exact(&mut buffer).await?;
        Ok((message_type, buffer))
    }

    pub async fn communicate(
        &mut self,
        message_type: &MessageType,
        message: &[u8],
    ) -> Result<(MessageType, serde_json::Value), anyhow::Error> {
        self.send_message(message_type, message).await?;
        let (message_type, response) = self.receive_message().await?;
        Ok((
            message_type,
            serde_json::from_str(String::from_utf8_lossy(response.as_ref()).as_ref())?,
        ))
    }

    pub async fn subscribe(
        &mut self,
        events: &[MessageType],
        callback: &'a SubscriptionCallback,
    ) -> Result<(), anyhow::Error> {
        let json = serde_json::to_string(events)?;
        let (message_type, response) = self
            .communicate(&MessageType::Subscribe, json.as_bytes())
            .await?;
        for s in events {
            self.subscriptions.insert(*s, Box::new(callback));
        }
        Ok(())
    }

    pub async fn call_callback(
        &mut self,
        subscription: &MessageType,
        response: serde_json::Value,
    ) -> Vec<(MessageType, Vec<u8>)> {
        let cb = self.subscriptions.get(subscription);
        if cb.is_none() {
            return Vec::new();
        }
        (*cb.unwrap())(*subscription, response).await
    }

    pub async fn run(&mut self) -> Result<(), anyhow::Error> {
        loop {
            let (message_type, response) = self.receive_message().await?;
            if !message_type.is_subscription() {
                continue;
            }

            let json_response =
                serde_json::from_str(String::from_utf8_lossy(response.as_ref()).as_ref())?;
            let messages: Vec<(MessageType, Vec<u8>)> =
                self.call_callback(&message_type, json_response).await;
            for (message_type, message) in messages {
                // TODO maybe log responses?
                self.communicate(&message_type, &message).await?;
            }
        }
    }
}

impl<'a> Clone for Connection<'a> {
    fn clone(&self) -> Self {
        let path: std::path::PathBuf = self
            .stream
            .get_ref()
            .peer_addr()
            .unwrap()
            .as_pathname()
            .unwrap()
            .into();
        Self::connect(path.as_ref()).unwrap()
    }
}
