use anyhow::{Context, Result};
use midir::{Ignore, MidiInput, MidiInputConnection};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use studio_core::{
    AccessMode, ClientCommand, DeviceState, MidiActivity, MidiEvent, PROTOCOL_VERSION,
    ServerMessage, Snapshot, detect_chord, now_ms, socket_path,
};

#[derive(Clone, Copy)]
struct DeviceSpec {
    id: &'static str,
    name: &'static str,
    midi_name_contains: &'static str,
}

const DEVICES: [DeviceSpec; 3] = [
    DeviceSpec {
        id: "novation.launchpad-original",
        name: "Novation Launchpad",
        midi_name_contains: "Launchpad",
    },
    DeviceSpec {
        id: "behringer.umc404hd",
        name: "Behringer UMC404HD",
        midi_name_contains: "UMC404HD",
    },
    DeviceSpec {
        id: "casio.usb-midi",
        name: "Casio USB-MIDI Keyboard",
        midi_name_contains: "CASIO USB-MIDI",
    },
];

struct IncomingMidi {
    device_id: String,
    bytes: Vec<u8>,
}

fn main() -> Result<()> {
    let snapshot = initial_snapshot();
    let state = Arc::new(Mutex::new(snapshot));
    let subscribers = Arc::new(Mutex::new(Vec::<UnixStream>::new()));
    let socket = socket_path();

    start_ipc_server(socket.clone(), Arc::clone(&state), Arc::clone(&subscribers))?;

    let (midi_tx, midi_rx) = mpsc::channel::<IncomingMidi>();
    eprintln!(
        "studio-control: observe daemon listening on {}",
        socket.display()
    );
    run_observer(state, subscribers, midi_tx, midi_rx)
}

fn initial_snapshot() -> Snapshot {
    let devices = DEVICES
        .iter()
        .map(|spec| {
            (
                spec.id.to_string(),
                DeviceState {
                    id: spec.id.to_string(),
                    name: spec.name.to_string(),
                    access_mode: AccessMode::Observe,
                    midi_connected: false,
                    midi_port: None,
                    activity: MidiActivity::default(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    Snapshot {
        protocol_version: PROTOCOL_VERSION,
        daemon_started_at_ms: now_ms(),
        devices,
    }
}

fn start_ipc_server(
    path: std::path::PathBuf,
    state: Arc<Mutex<Snapshot>>,
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create runtime directory {}", parent.display()))?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("remove stale socket {}", path.display()))?;
    }

    let listener =
        UnixListener::bind(&path).with_context(|| format!("bind IPC socket {}", path.display()))?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;

    thread::Builder::new()
        .name("studio-ipc".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let state = Arc::clone(&state);
                        let subscribers = Arc::clone(&subscribers);
                        thread::spawn(move || {
                            if let Err(error) = handle_client(stream, state, subscribers) {
                                eprintln!("studio-control: IPC client error: {error:#}");
                            }
                        });
                    }
                    Err(error) => eprintln!("studio-control: IPC accept error: {error}"),
                }
            }
        })?;
    Ok(())
}

fn handle_client(
    mut stream: UnixStream,
    state: Arc<Mutex<Snapshot>>,
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
) -> Result<()> {
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    let command: ClientCommand =
        serde_json::from_str(line.trim()).context("parse client command")?;

    match command.command.as_str() {
        "status" => {
            write_message(
                &mut stream,
                &ServerMessage::Snapshot {
                    snapshot: state.lock().expect("state lock").clone(),
                },
            )?;
        }
        "watch" => {
            write_message(
                &mut stream,
                &ServerMessage::Snapshot {
                    snapshot: state.lock().expect("state lock").clone(),
                },
            )?;
            subscribers.lock().expect("subscriber lock").push(stream);
        }
        other => {
            write_message(
                &mut stream,
                &ServerMessage::Error {
                    message: format!("unknown command: {other}"),
                },
            )?;
        }
    }
    Ok(())
}

fn write_message(stream: &mut UnixStream, message: &ServerMessage) -> Result<()> {
    serde_json::to_writer(&mut *stream, message)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn broadcast(subscribers: &Arc<Mutex<Vec<UnixStream>>>, message: &ServerMessage) {
    let Ok(mut bytes) = serde_json::to_vec(message) else {
        return;
    };
    bytes.push(b'\n');

    subscribers
        .lock()
        .expect("subscriber lock")
        .retain_mut(|stream| {
            stream
                .write_all(&bytes)
                .and_then(|_| stream.flush())
                .is_ok()
        });
}

fn run_observer(
    state: Arc<Mutex<Snapshot>>,
    subscribers: Arc<Mutex<Vec<UnixStream>>>,
    midi_tx: Sender<IncomingMidi>,
    midi_rx: Receiver<IncomingMidi>,
) -> Result<()> {
    let mut connections = HashMap::<String, MidiInputConnection<()>>::new();
    let mut last_reconcile = Instant::now() - Duration::from_secs(5);

    loop {
        if last_reconcile.elapsed() >= Duration::from_secs(1) {
            if reconcile_connections(&mut connections, &state, &midi_tx)? {
                let message = ServerMessage::Snapshot {
                    snapshot: state.lock().expect("state lock").clone(),
                };
                broadcast(&subscribers, &message);
            }
            last_reconcile = Instant::now();
        }

        if let Ok(incoming) = midi_rx.recv_timeout(Duration::from_millis(40)) {
            let (event, activity) = apply_midi_event(&state, incoming);
            broadcast(
                &subscribers,
                &ServerMessage::MidiEvent {
                    event,
                    activity: Box::new(activity),
                },
            );
        }
    }
}

fn available_port_names() -> Result<Vec<String>> {
    let input = MidiInput::new("studio-control-discovery")?;
    Ok(input
        .ports()
        .iter()
        .filter_map(|port| input.port_name(port).ok())
        .collect())
}

fn reconcile_connections(
    connections: &mut HashMap<String, MidiInputConnection<()>>,
    state: &Arc<Mutex<Snapshot>>,
    midi_tx: &Sender<IncomingMidi>,
) -> Result<bool> {
    let available = available_port_names()?;
    let mut changed = false;

    for spec in DEVICES {
        let matched_name = available
            .iter()
            .find(|name| name.contains(spec.midi_name_contains))
            .cloned();

        if matched_name.is_none() && connections.remove(spec.id).is_some() {
            changed = true;
        }

        if let Some(port_name) = matched_name.as_ref()
            && !connections.contains_key(spec.id)
        {
            match connect_input(spec, port_name, midi_tx.clone()) {
                Ok(connection) => {
                    connections.insert(spec.id.to_string(), connection);
                    changed = true;
                    eprintln!("studio-control: observing {port_name}");
                }
                Err(error) => {
                    eprintln!("studio-control: could not observe {port_name}: {error:#}");
                }
            }
        }

        let connected = connections.contains_key(spec.id);
        let mut snapshot = state.lock().expect("state lock");
        if let Some(device) = snapshot.devices.get_mut(spec.id)
            && (device.midi_connected != connected || device.midi_port != matched_name)
        {
            device.midi_connected = connected;
            device.midi_port = matched_name;
            changed = true;
        }
    }

    Ok(changed)
}

fn connect_input(
    spec: DeviceSpec,
    expected_port_name: &str,
    midi_tx: Sender<IncomingMidi>,
) -> Result<MidiInputConnection<()>> {
    let mut input = MidiInput::new(&format!("studio-control-{}", spec.id))?;
    input.ignore(Ignore::None);
    let port = input
        .ports()
        .into_iter()
        .find(|port| {
            input
                .port_name(port)
                .is_ok_and(|name| name == expected_port_name)
        })
        .with_context(|| format!("MIDI port disappeared: {expected_port_name}"))?;
    let device_id = spec.id.to_string();

    input
        .connect(
            &port,
            &format!("observe-{}", spec.id),
            move |_timestamp, message, _| {
                let _ = midi_tx.send(IncomingMidi {
                    device_id: device_id.clone(),
                    bytes: message.to_vec(),
                });
            },
            (),
        )
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn apply_midi_event(
    state: &Arc<Mutex<Snapshot>>,
    incoming: IncomingMidi,
) -> (MidiEvent, MidiActivity) {
    let event = parse_midi_event(incoming.device_id, incoming.bytes);
    let mut snapshot = state.lock().expect("state lock");
    let device = snapshot
        .devices
        .get_mut(&event.device_id)
        .expect("known MIDI device");

    match event.kind.as_str() {
        "note_on" => {
            if let Some(note) = event.note
                && !device.activity.active_notes.contains(&note)
            {
                device.activity.active_notes.push(note);
                device.activity.active_notes.sort_unstable();
            }
        }
        "note_off" => {
            if let Some(note) = event.note {
                device
                    .activity
                    .active_notes
                    .retain(|active| *active != note);
            }
        }
        "control_change" if event.controller == Some(64) => {
            device.activity.sustain = event.value.unwrap_or(0) >= 64;
        }
        _ => {}
    }

    device.activity.event_count += 1;
    device.activity.last_event = Some(event.clone());
    device.activity.chord = detect_chord(&device.activity.active_notes);
    if device.activity.chord.is_some() {
        device.activity.last_chord = device.activity.chord.clone();
    }
    (event, device.activity.clone())
}

fn parse_midi_event(device_id: String, raw: Vec<u8>) -> MidiEvent {
    let status = raw.first().copied().unwrap_or(0);
    let command = status & 0xf0;
    let channel = (status < 0xf0).then_some(status & 0x0f);
    let data1 = raw.get(1).copied();
    let data2 = raw.get(2).copied();

    let (kind, note, velocity, controller, value) = match command {
        0x80 => ("note_off", data1, data2, None, None),
        0x90 if data2.unwrap_or(0) == 0 => ("note_off", data1, data2, None, None),
        0x90 => ("note_on", data1, data2, None, None),
        0xb0 => ("control_change", None, None, data1, data2),
        0xc0 => ("program_change", None, None, None, data1),
        0xe0 => ("pitch_bend", None, None, None, data2),
        _ if status >= 0xf0 => ("system", None, None, None, None),
        _ => ("other", None, None, None, None),
    };

    MidiEvent {
        device_id,
        timestamp_ms: now_ms(),
        kind: kind.to_string(),
        channel,
        note,
        velocity,
        controller,
        value,
        raw,
    }
}

#[allow(dead_code)]
fn remove_socket(path: &Path) {
    let _ = fs::remove_file(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(raw: &[u8]) -> MidiEvent {
        parse_midi_event("test.device".to_string(), raw.to_vec())
    }

    #[test]
    fn parses_note_on_and_velocity_zero_note_off() {
        let note_on = parsed(&[0x92, 64, 101]);
        assert_eq!(note_on.kind, "note_on");
        assert_eq!(note_on.channel, Some(2));
        assert_eq!(note_on.note, Some(64));
        assert_eq!(note_on.velocity, Some(101));

        let note_off = parsed(&[0x92, 64, 0]);
        assert_eq!(note_off.kind, "note_off");
        assert_eq!(note_off.note, Some(64));
    }

    #[test]
    fn parses_sustain_control_change() {
        let event = parsed(&[0xb0, 64, 127]);
        assert_eq!(event.kind, "control_change");
        assert_eq!(event.controller, Some(64));
        assert_eq!(event.value, Some(127));
    }

    #[test]
    fn activity_tracks_active_notes_and_sustain() {
        let state = Arc::new(Mutex::new(initial_snapshot()));

        apply_midi_event(
            &state,
            IncomingMidi {
                device_id: "casio.usb-midi".to_string(),
                bytes: vec![0x90, 60, 90],
            },
        );
        apply_midi_event(
            &state,
            IncomingMidi {
                device_id: "casio.usb-midi".to_string(),
                bytes: vec![0xb0, 64, 127],
            },
        );

        let snapshot = state.lock().expect("state lock");
        let activity = &snapshot.devices["casio.usb-midi"].activity;
        assert_eq!(activity.active_notes, vec![60]);
        assert!(activity.sustain);
        assert_eq!(activity.event_count, 2);
    }

    #[test]
    fn activity_tracks_current_and_last_chord() {
        let state = Arc::new(Mutex::new(initial_snapshot()));
        for note in [60, 64, 67] {
            apply_midi_event(
                &state,
                IncomingMidi {
                    device_id: "casio.usb-midi".to_string(),
                    bytes: vec![0x90, note, 90],
                },
            );
        }

        apply_midi_event(
            &state,
            IncomingMidi {
                device_id: "casio.usb-midi".to_string(),
                bytes: vec![0x80, 60, 64],
            },
        );

        let snapshot = state.lock().expect("state lock");
        let activity = &snapshot.devices["casio.usb-midi"].activity;
        assert_eq!(activity.chord, None);
        assert_eq!(activity.last_chord.as_deref(), Some("C"));
    }
}
