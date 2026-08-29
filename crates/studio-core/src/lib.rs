use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    Released,
    #[default]
    Observe,
    Managed,
    Exclusive,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiEvent {
    pub device_id: String,
    pub timestamp_ms: u64,
    pub kind: String,
    pub channel: Option<u8>,
    pub note: Option<u8>,
    pub velocity: Option<u8>,
    pub controller: Option<u8>,
    pub value: Option<u8>,
    pub raw: Vec<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiActivity {
    pub active_notes: Vec<u8>,
    pub sustain: bool,
    pub event_count: u64,
    pub last_event: Option<MidiEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    pub id: String,
    pub name: String,
    pub access_mode: AccessMode,
    pub midi_connected: bool,
    pub midi_port: Option<String>,
    pub activity: MidiActivity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub protocol_version: u32,
    pub daemon_started_at_ms: u64,
    pub devices: BTreeMap<String, DeviceState>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Snapshot {
        snapshot: Snapshot,
    },
    MidiEvent {
        event: MidiEvent,
        activity: MidiActivity,
    },
    Error {
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientCommand {
    pub command: String,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

pub fn socket_path() -> PathBuf {
    if let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir)
            .join("studio-control")
            .join("control.sock");
    }

    let user = env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    env::temp_dir().join(format!("studio-control-{user}.sock"))
}
