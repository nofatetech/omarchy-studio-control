use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: u32 = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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
    pub active_controls: Vec<u8>,
    pub sustain: bool,
    pub event_count: u64,
    pub last_event: Option<MidiEvent>,
    pub chord: Option<String>,
    pub last_chord: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiPeer {
    pub client_id: u8,
    pub port_id: u8,
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiPeers {
    pub input_consumers: Vec<MidiPeer>,
    pub output_producers: Vec<MidiPeer>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSurfaceState {
    pub rows: u8,
    pub columns: u8,
    pub cells: Vec<u8>,
    pub animation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceState {
    pub id: String,
    pub name: String,
    pub access_mode: AccessMode,
    pub midi_connected: bool,
    pub midi_port: Option<String>,
    pub midi_peers: MidiPeers,
    pub activity: MidiActivity,
    pub surface: Option<GridSurfaceState>,
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
        activity: Box<MidiActivity>,
    },
    Error {
        message: String,
    },
    CommandResult {
        ok: bool,
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCommand {
    pub command: String,
    pub device_id: Option<String>,
    pub action: Option<String>,
    pub payload: Option<serde_json::Value>,
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

pub fn detect_chord(notes: &[u8]) -> Option<String> {
    const PATTERNS: &[(&str, &[u8])] = &[
        ("maj9", &[0, 2, 4, 7, 11]),
        ("9", &[0, 2, 4, 7, 10]),
        ("m9", &[0, 2, 3, 7, 10]),
        ("add9", &[0, 2, 4, 7]),
        ("m(add9)", &[0, 2, 3, 7]),
        ("6", &[0, 4, 7, 9]),
        ("m6", &[0, 3, 7, 9]),
        ("maj7", &[0, 4, 7, 11]),
        ("7", &[0, 4, 7, 10]),
        ("m7", &[0, 3, 7, 10]),
        ("m(maj7)", &[0, 3, 7, 11]),
        ("dim7", &[0, 3, 6, 9]),
        ("m7♭5", &[0, 3, 6, 10]),
        ("", &[0, 4, 7]),
        ("m", &[0, 3, 7]),
        ("dim", &[0, 3, 6]),
        ("aug", &[0, 4, 8]),
        ("sus2", &[0, 2, 7]),
        ("sus4", &[0, 5, 7]),
        ("5", &[0, 7]),
    ];
    const NAMES: [&str; 12] = [
        "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
    ];

    let bass = notes.iter().min().copied()? % 12;
    let mut pitch_classes = notes.iter().map(|note| note % 12).collect::<Vec<_>>();
    pitch_classes.sort_unstable();
    pitch_classes.dedup();
    if pitch_classes.len() < 2 {
        return None;
    }

    let mut roots = vec![bass];
    roots.extend((0..12).filter(|root| *root != bass));

    for root in roots {
        let mut intervals = pitch_classes
            .iter()
            .map(|pitch| (pitch + 12 - root) % 12)
            .collect::<Vec<_>>();
        intervals.sort_unstable();

        if let Some((suffix, _)) = PATTERNS
            .iter()
            .find(|(_, pattern)| intervals.as_slice() == *pattern)
        {
            let mut chord = format!("{}{}", NAMES[root as usize], suffix);
            if bass != root {
                chord.push('/');
                chord.push_str(NAMES[bass as usize]);
            }
            return Some(chord);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::detect_chord;

    #[test]
    fn detects_triads_sevenths_and_inversions() {
        assert_eq!(detect_chord(&[60, 64, 67]).as_deref(), Some("C"));
        assert_eq!(detect_chord(&[60, 63, 67, 70]).as_deref(), Some("Cm7"));
        assert_eq!(detect_chord(&[64, 67, 72]).as_deref(), Some("C/E"));
        assert_eq!(detect_chord(&[57, 60, 64, 67]).as_deref(), Some("Am7"));
    }

    #[test]
    fn ignores_single_notes_and_unknown_sets() {
        assert_eq!(detect_chord(&[60]), None);
        assert_eq!(detect_chord(&[60, 61, 66]), None);
    }
}
