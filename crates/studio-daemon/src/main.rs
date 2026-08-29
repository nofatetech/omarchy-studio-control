use anyhow::{Context, Result};
use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use studio_core::{
    AccessMode, ClientCommand, DeviceState, GridSurfaceState, MidiActivity, MidiEvent, MidiPeer,
    MidiPeers, PROTOCOL_VERSION, ServerMessage, Snapshot, detect_chord, now_ms, socket_path,
};

#[derive(Clone, Copy)]
struct DeviceSpec {
    id: &'static str,
    name: &'static str,
    midi_name_contains: &'static str,
    seq_client_contains: &'static str,
}

const DEVICES: [DeviceSpec; 3] = [
    DeviceSpec {
        id: "novation.launchpad-original",
        name: "Novation Launchpad",
        midi_name_contains: "Launchpad",
        seq_client_contains: "Launchpad",
    },
    DeviceSpec {
        id: "behringer.umc404hd",
        name: "Behringer UMC404HD",
        midi_name_contains: "UMC404HD",
        seq_client_contains: "UMC404HD",
    },
    DeviceSpec {
        id: "casio.usb-midi",
        name: "Casio USB-MIDI Keyboard",
        midi_name_contains: "CASIO USB-MIDI",
        seq_client_contains: "CASIO USB-MIDI",
    },
];

struct IncomingMidi {
    device_id: String,
    bytes: Vec<u8>,
}

struct ControlRequest {
    command: ClientCommand,
    reply: Sender<ServerMessage>,
}

struct ScrollAnimation {
    columns: Vec<u8>,
    offset: usize,
    color: u8,
    next_frame: Instant,
}

#[derive(Default)]
struct SeqPort {
    connecting_to: Vec<(u8, u8)>,
    connected_from: Vec<(u8, u8)>,
}

struct SeqClient {
    id: u8,
    name: String,
    ports: HashMap<u8, SeqPort>,
}

fn main() -> Result<()> {
    let snapshot = initial_snapshot();
    let state = Arc::new(Mutex::new(snapshot));
    let subscribers = Arc::new(Mutex::new(Vec::<UnixStream>::new()));
    let socket = socket_path();

    let (control_tx, control_rx) = mpsc::channel::<ControlRequest>();
    start_ipc_server(
        socket.clone(),
        Arc::clone(&state),
        Arc::clone(&subscribers),
        control_tx,
    )?;

    let (midi_tx, midi_rx) = mpsc::channel::<IncomingMidi>();
    eprintln!(
        "studio-control: observe daemon listening on {}",
        socket.display()
    );
    run_observer(state, subscribers, midi_tx, midi_rx, control_rx)
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
                    midi_peers: MidiPeers::default(),
                    surface: (spec.id == "novation.launchpad-original").then(|| GridSurfaceState {
                        rows: 8,
                        columns: 8,
                        cells: vec![0; 64],
                        animation: None,
                    }),
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
    control_tx: Sender<ControlRequest>,
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
                        let control_tx = control_tx.clone();
                        thread::spawn(move || {
                            if let Err(error) =
                                handle_client(stream, state, subscribers, control_tx)
                            {
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
    control_tx: Sender<ControlRequest>,
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
        "device_command" => {
            let (reply_tx, reply_rx) = mpsc::channel();
            control_tx
                .send(ControlRequest {
                    command,
                    reply: reply_tx,
                })
                .context("send device command")?;
            let response = reply_rx
                .recv_timeout(Duration::from_secs(2))
                .context("device command timed out")?;
            write_message(&mut stream, &response)?;
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
    control_rx: Receiver<ControlRequest>,
) -> Result<()> {
    let mut connections = HashMap::<String, MidiInputConnection<()>>::new();
    let mut launchpad_output: Option<MidiOutputConnection> = None;
    let mut launchpad_animation: Option<ScrollAnimation> = None;
    let mut last_reconcile = Instant::now() - Duration::from_secs(5);

    loop {
        if last_reconcile.elapsed() >= Duration::from_secs(1) {
            let mut changed = reconcile_connections(&mut connections, &state, &midi_tx)?;
            let launchpad_connected = state
                .lock()
                .expect("state lock")
                .devices
                .get("novation.launchpad-original")
                .is_some_and(|device| device.midi_connected);
            if !launchpad_connected && launchpad_output.is_some() {
                launchpad_output = None;
                launchpad_animation = None;
                let mut snapshot = state.lock().expect("state lock");
                let launchpad = snapshot
                    .devices
                    .get_mut("novation.launchpad-original")
                    .expect("Launchpad state");
                launchpad.access_mode = AccessMode::Observe;
                if let Some(surface) = launchpad.surface.as_mut() {
                    surface.cells.fill(0);
                    surface.animation = None;
                }
                changed = true;
            }
            if changed {
                let message = ServerMessage::Snapshot {
                    snapshot: state.lock().expect("state lock").clone(),
                };
                broadcast(&subscribers, &message);
            }
            last_reconcile = Instant::now();
        }

        while let Ok(request) = control_rx.try_recv() {
            let result = execute_device_command(
                &request.command,
                &state,
                &mut launchpad_output,
                &mut launchpad_animation,
            );
            let response = match result {
                Ok(message) => {
                    broadcast_snapshot(&state, &subscribers);
                    ServerMessage::CommandResult { ok: true, message }
                }
                Err(error) => ServerMessage::CommandResult {
                    ok: false,
                    message: format!("{error:#}"),
                },
            };
            let _ = request.reply.send(response);
        }

        if let Ok(incoming) = midi_rx.recv_timeout(Duration::from_millis(40)) {
            let (event, activity) = apply_midi_event(&state, incoming);
            let mut surface_changed = false;
            if event.device_id == "novation.launchpad-original" && event.kind == "note_on" {
                match handle_launchpad_press(
                    &event,
                    &state,
                    &mut launchpad_output,
                    &mut launchpad_animation,
                ) {
                    Ok(changed) => surface_changed = changed,
                    Err(error) => {
                        eprintln!("studio-control: Launchpad input action failed: {error:#}")
                    }
                }
            }
            broadcast(
                &subscribers,
                &ServerMessage::MidiEvent {
                    event,
                    activity: Box::new(activity),
                },
            );
            if surface_changed {
                broadcast_snapshot(&state, &subscribers);
            }
        }

        let animation_due = launchpad_animation
            .as_ref()
            .is_some_and(|animation| Instant::now() >= animation.next_frame);
        if animation_due {
            match advance_animation(&state, &mut launchpad_output, &mut launchpad_animation) {
                Ok(()) => broadcast_snapshot(&state, &subscribers),
                Err(error) => {
                    eprintln!("studio-control: Launchpad animation failed: {error:#}");
                    launchpad_animation = None;
                }
            }
        }
    }
}

fn broadcast_snapshot(state: &Arc<Mutex<Snapshot>>, subscribers: &Arc<Mutex<Vec<UnixStream>>>) {
    broadcast(
        subscribers,
        &ServerMessage::Snapshot {
            snapshot: state.lock().expect("state lock").clone(),
        },
    );
}

fn available_port_names() -> Result<Vec<String>> {
    let input = MidiInput::new("studio-control-discovery")?;
    Ok(input
        .ports()
        .iter()
        .filter_map(|port| input.port_name(port).ok())
        .collect())
}

fn parse_seq_clients(text: &str) -> Vec<SeqClient> {
    let mut clients = Vec::<SeqClient>::new();
    let mut current_client: Option<SeqClient> = None;
    let mut current_port: Option<u8> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Client ") {
            if let Some(client) = current_client.take() {
                clients.push(client);
            }
            let Some((id_text, name_text)) = rest.split_once(':') else {
                continue;
            };
            let Ok(id) = id_text.trim().parse::<u8>() else {
                continue;
            };
            let name = name_text.split('"').nth(1).unwrap_or_default().to_string();
            current_client = Some(SeqClient {
                id,
                name,
                ports: HashMap::new(),
            });
            current_port = None;
        } else if let Some(rest) = trimmed.strip_prefix("Port ") {
            let Some((id_text, _)) = rest.split_once(':') else {
                continue;
            };
            let Ok(id) = id_text.trim().parse::<u8>() else {
                continue;
            };
            if let Some(client) = current_client.as_mut() {
                client.ports.entry(id).or_default();
                current_port = Some(id);
            }
        } else if let Some(rest) = trimmed.strip_prefix("Connecting To:") {
            if let (Some(client), Some(port)) = (current_client.as_mut(), current_port) {
                client.ports.entry(port).or_default().connecting_to = parse_endpoints(rest);
            }
        } else if let Some(rest) = trimmed.strip_prefix("Connected From:")
            && let (Some(client), Some(port)) = (current_client.as_mut(), current_port)
        {
            client.ports.entry(port).or_default().connected_from = parse_endpoints(rest);
        }
    }

    if let Some(client) = current_client {
        clients.push(client);
    }
    clients
}

fn parse_endpoints(text: &str) -> Vec<(u8, u8)> {
    text.split(',')
        .filter_map(|endpoint| {
            let (client, port) = endpoint.trim().split_once(':')?;
            Some((client.parse().ok()?, port.parse().ok()?))
        })
        .collect()
}

fn discover_midi_peers() -> Result<HashMap<String, MidiPeers>> {
    let clients = parse_seq_clients(
        &fs::read_to_string("/proc/asound/seq/clients").context("read ALSA sequencer clients")?,
    );
    let names = clients
        .iter()
        .map(|client| (client.id, client.name.clone()))
        .collect::<HashMap<_, _>>();
    let mut result = HashMap::new();

    for spec in DEVICES {
        let Some(client) = clients
            .iter()
            .find(|client| client.name.contains(spec.seq_client_contains))
        else {
            continue;
        };
        let Some(port) = client.ports.get(&0) else {
            continue;
        };
        let make_peer = |(client_id, port_id): &(u8, u8)| MidiPeer {
            client_id: *client_id,
            port_id: *port_id,
            name: names
                .get(client_id)
                .cloned()
                .unwrap_or_else(|| format!("ALSA client {client_id}")),
        };
        let external = |peer: &MidiPeer| !peer.name.starts_with("studio-control-");
        let mut input_consumers = port
            .connecting_to
            .iter()
            .map(make_peer)
            .filter(external)
            .collect::<Vec<_>>();
        let mut output_producers = port
            .connected_from
            .iter()
            .map(make_peer)
            .filter(external)
            .collect::<Vec<_>>();
        input_consumers.sort_by_key(|peer| (peer.client_id, peer.port_id));
        output_producers.sort_by_key(|peer| (peer.client_id, peer.port_id));
        result.insert(
            spec.id.to_string(),
            MidiPeers {
                input_consumers,
                output_producers,
            },
        );
    }
    Ok(result)
}

fn reconcile_connections(
    connections: &mut HashMap<String, MidiInputConnection<()>>,
    state: &Arc<Mutex<Snapshot>>,
    midi_tx: &Sender<IncomingMidi>,
) -> Result<bool> {
    let available = available_port_names()?;
    let peers = discover_midi_peers().unwrap_or_default();
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
        if let Some(device) = snapshot.devices.get_mut(spec.id) {
            let next_peers = peers.get(spec.id).cloned().unwrap_or_default();
            if device.midi_connected != connected
                || device.midi_port != matched_name
                || device.midi_peers != next_peers
            {
                device.midi_connected = connected;
                device.midi_port = matched_name;
                device.midi_peers = next_peers;
                changed = true;
            }
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

fn connect_output(expected_port_name: &str) -> Result<MidiOutputConnection> {
    let output = MidiOutput::new("studio-control-launchpad-leds")?;
    let port = output
        .ports()
        .into_iter()
        .find(|port| {
            output
                .port_name(port)
                .is_ok_and(|name| name == expected_port_name)
        })
        .with_context(|| format!("MIDI output disappeared: {expected_port_name}"))?;
    output
        .connect(&port, "managed-launchpad-leds")
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn execute_device_command(
    command: &ClientCommand,
    state: &Arc<Mutex<Snapshot>>,
    output: &mut Option<MidiOutputConnection>,
    animation: &mut Option<ScrollAnimation>,
) -> Result<String> {
    let device_id = command.device_id.as_deref().context("missing deviceId")?;
    let action = command.action.as_deref().context("missing action")?;
    let payload = command.payload.as_ref().unwrap_or(&Value::Null);
    if device_id != "novation.launchpad-original" {
        anyhow::bail!("{device_id} does not expose managed controls yet");
    }

    if action == "set_mode" {
        let mode = payload
            .get("mode")
            .and_then(Value::as_str)
            .context("set_mode requires a mode")?;
        return match mode {
            "managed" => {
                let discovered = discover_midi_peers()
                    .ok()
                    .and_then(|peers| peers.get(device_id).cloned())
                    .unwrap_or_default();
                if !discovered.output_producers.is_empty() {
                    let names = discovered
                        .output_producers
                        .iter()
                        .map(|peer| peer.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    anyhow::bail!("LED output is already in use by {names}");
                }
                let port_name = state
                    .lock()
                    .expect("state lock")
                    .devices
                    .get(device_id)
                    .and_then(|device| device.midi_port.clone())
                    .context("Launchpad is not connected")?;
                let mut connection = connect_output(&port_name)?;
                connection
                    .send(&[0xb0, 0, 0])
                    .map_err(|error| anyhow::anyhow!(error.to_string()))?;
                *output = Some(connection);
                *animation = None;
                let mut snapshot = state.lock().expect("state lock");
                let device = snapshot
                    .devices
                    .get_mut(device_id)
                    .expect("Launchpad state");
                device.access_mode = AccessMode::Managed;
                if let Some(surface) = device.surface.as_mut() {
                    surface.cells.fill(0);
                    surface.animation = None;
                }
                Ok("Launchpad LED management enabled".to_string())
            }
            "observe" | "released" => {
                if let Some(connection) = output.as_mut() {
                    let _ = connection.send(&[0xb0, 0, 0]);
                }
                *output = None;
                *animation = None;
                let mut snapshot = state.lock().expect("state lock");
                let device = snapshot
                    .devices
                    .get_mut(device_id)
                    .expect("Launchpad state");
                device.access_mode = AccessMode::Observe;
                if let Some(surface) = device.surface.as_mut() {
                    surface.cells.fill(0);
                    surface.animation = None;
                }
                Ok("Launchpad returned to observe-only mode".to_string())
            }
            other => anyhow::bail!("unsupported access mode: {other}"),
        };
    }

    let managed = state
        .lock()
        .expect("state lock")
        .devices
        .get(device_id)
        .is_some_and(|device| device.access_mode == AccessMode::Managed);
    if !managed || output.is_none() {
        anyhow::bail!("enable managed mode before sending LED commands");
    }

    match action {
        "set_pad" => {
            let x = payload_u8(payload, "x")?;
            let y = payload_u8(payload, "y")?;
            let color = payload_u8(payload, "color")?;
            if x >= 8 || y >= 8 || color > 4 {
                anyhow::bail!("pad x/y must be 0..7 and color must be 0..4");
            }
            send_pad(output.as_mut().expect("managed output"), x, y, color)?;
            *animation = None;
            let mut snapshot = state.lock().expect("state lock");
            let surface = snapshot
                .devices
                .get_mut(device_id)
                .unwrap()
                .surface
                .as_mut()
                .unwrap();
            surface.cells[usize::from(y) * 8 + usize::from(x)] = color;
            surface.animation = None;
            Ok(format!("set pad {x},{y}"))
        }
        "clear" => {
            send_frame(output.as_mut().expect("managed output"), &[0; 64])?;
            *animation = None;
            let mut snapshot = state.lock().expect("state lock");
            let surface = snapshot
                .devices
                .get_mut(device_id)
                .unwrap()
                .surface
                .as_mut()
                .unwrap();
            surface.cells.fill(0);
            surface.animation = None;
            Ok("cleared Launchpad LEDs".to_string())
        }
        "frame" => {
            let cells = payload
                .get("cells")
                .and_then(Value::as_array)
                .context("frame requires a cells array")?
                .iter()
                .map(|value| value.as_u64().and_then(|value| u8::try_from(value).ok()))
                .collect::<Option<Vec<_>>>()
                .context("frame cells must be integers")?;
            if cells.len() != 64 || cells.iter().any(|color| *color > 4) {
                anyhow::bail!("frame requires exactly 64 color values in the range 0..4");
            }
            send_frame(output.as_mut().expect("managed output"), &cells)?;
            *animation = None;
            let mut snapshot = state.lock().expect("state lock");
            let surface = snapshot
                .devices
                .get_mut(device_id)
                .unwrap()
                .surface
                .as_mut()
                .unwrap();
            surface.cells = cells;
            surface.animation = None;
            Ok("painted Launchpad frame".to_string())
        }
        "scroll" => {
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("STUDIO")
                .trim();
            if text.is_empty() {
                anyhow::bail!("scroll text cannot be empty");
            }
            let color = payload
                .get("color")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .unwrap_or(3);
            if color == 0 || color > 4 {
                anyhow::bail!("scroll color must be 1..4");
            }
            *animation = Some(ScrollAnimation {
                columns: text_columns(text),
                offset: 0,
                color,
                next_frame: Instant::now(),
            });
            state
                .lock()
                .expect("state lock")
                .devices
                .get_mut(device_id)
                .unwrap()
                .surface
                .as_mut()
                .unwrap()
                .animation = Some(format!("Scrolling “{text}”"));
            Ok(format!("scrolling “{text}”"))
        }
        "stop" => {
            *animation = None;
            state
                .lock()
                .expect("state lock")
                .devices
                .get_mut(device_id)
                .unwrap()
                .surface
                .as_mut()
                .unwrap()
                .animation = None;
            Ok("stopped Launchpad animation".to_string())
        }
        other => anyhow::bail!("unsupported Launchpad action: {other}"),
    }
}

fn payload_u8(payload: &Value, key: &str) -> Result<u8> {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .with_context(|| format!("missing or invalid {key}"))
}

fn launchpad_velocity(color: u8) -> u8 {
    match color {
        1 => 60,
        2 => 15,
        3 => 63,
        4 => 62,
        _ => 12,
    }
}

fn send_pad(connection: &mut MidiOutputConnection, x: u8, y: u8, color: u8) -> Result<()> {
    connection
        .send(&[0x90, y * 16 + x, launchpad_velocity(color)])
        .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn send_frame(connection: &mut MidiOutputConnection, cells: &[u8]) -> Result<()> {
    for (index, color) in cells.iter().enumerate() {
        send_pad(
            connection,
            u8::try_from(index % 8).unwrap(),
            u8::try_from(index / 8).unwrap(),
            *color,
        )?;
    }
    Ok(())
}

fn handle_launchpad_press(
    event: &MidiEvent,
    state: &Arc<Mutex<Snapshot>>,
    output: &mut Option<MidiOutputConnection>,
    animation: &mut Option<ScrollAnimation>,
) -> Result<bool> {
    let Some(note) = event.note else {
        return Ok(false);
    };
    let x = note % 16;
    let y = note / 16;
    if x >= 8 || y >= 8 || output.is_none() {
        return Ok(false);
    }
    let mut snapshot = state.lock().expect("state lock");
    let device = snapshot
        .devices
        .get_mut("novation.launchpad-original")
        .expect("Launchpad state");
    if device.access_mode != AccessMode::Managed {
        return Ok(false);
    }
    let surface = device.surface.as_mut().expect("Launchpad surface");
    let index = usize::from(y) * 8 + usize::from(x);
    let color = (surface.cells[index] + 1) % 5;
    send_pad(output.as_mut().expect("managed output"), x, y, color)?;
    surface.cells[index] = color;
    surface.animation = None;
    *animation = None;
    Ok(true)
}

fn advance_animation(
    state: &Arc<Mutex<Snapshot>>,
    output: &mut Option<MidiOutputConnection>,
    animation: &mut Option<ScrollAnimation>,
) -> Result<()> {
    let animation = animation.as_mut().context("animation disappeared")?;
    let mut next = vec![0; 64];
    for x in 0..8 {
        let column = animation.columns[(animation.offset + x) % animation.columns.len()];
        for y in 0..7 {
            if column & (1 << y) != 0 {
                next[y * 8 + x] = animation.color;
            }
        }
    }

    let mut snapshot = state.lock().expect("state lock");
    let surface = snapshot
        .devices
        .get_mut("novation.launchpad-original")
        .unwrap()
        .surface
        .as_mut()
        .unwrap();
    let connection = output.as_mut().context("Launchpad output is closed")?;
    for (index, next_color) in next.iter().enumerate() {
        if surface.cells[index] != *next_color {
            send_pad(
                connection,
                u8::try_from(index % 8).unwrap(),
                u8::try_from(index / 8).unwrap(),
                *next_color,
            )?;
        }
    }
    surface.cells = next;
    animation.offset = (animation.offset + 1) % animation.columns.len();
    animation.next_frame = Instant::now() + Duration::from_millis(175);
    Ok(())
}

fn text_columns(text: &str) -> Vec<u8> {
    let mut columns = vec![0; 8];
    for character in text.to_uppercase().chars() {
        columns.extend(glyph(character));
        columns.push(0);
    }
    columns.extend([0; 8]);
    columns
}

fn glyph(character: char) -> [u8; 5] {
    match character {
        'A' => [0x7e, 0x11, 0x11, 0x11, 0x7e],
        'B' => [0x7f, 0x49, 0x49, 0x49, 0x36],
        'C' => [0x3e, 0x41, 0x41, 0x41, 0x22],
        'D' => [0x7f, 0x41, 0x41, 0x22, 0x1c],
        'E' => [0x7f, 0x49, 0x49, 0x49, 0x41],
        'F' => [0x7f, 0x09, 0x09, 0x09, 0x01],
        'G' => [0x3e, 0x41, 0x49, 0x49, 0x7a],
        'H' => [0x7f, 0x08, 0x08, 0x08, 0x7f],
        'I' => [0x00, 0x41, 0x7f, 0x41, 0x00],
        'J' => [0x20, 0x40, 0x41, 0x3f, 0x01],
        'K' => [0x7f, 0x08, 0x14, 0x22, 0x41],
        'L' => [0x7f, 0x40, 0x40, 0x40, 0x40],
        'M' => [0x7f, 0x02, 0x0c, 0x02, 0x7f],
        'N' => [0x7f, 0x04, 0x08, 0x10, 0x7f],
        'O' => [0x3e, 0x41, 0x41, 0x41, 0x3e],
        'P' => [0x7f, 0x09, 0x09, 0x09, 0x06],
        'Q' => [0x3e, 0x41, 0x51, 0x21, 0x5e],
        'R' => [0x7f, 0x09, 0x19, 0x29, 0x46],
        'S' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' => [0x01, 0x01, 0x7f, 0x01, 0x01],
        'U' => [0x3f, 0x40, 0x40, 0x40, 0x3f],
        'V' => [0x1f, 0x20, 0x40, 0x20, 0x1f],
        'W' => [0x3f, 0x40, 0x38, 0x40, 0x3f],
        'X' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        '0' => [0x3e, 0x51, 0x49, 0x45, 0x3e],
        '1' => [0x00, 0x42, 0x7f, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4b, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7f, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3c, 0x4a, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1e],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        '!' => [0x00, 0x00, 0x5f, 0x00, 0x00],
        ' ' => [0; 5],
        _ => [0x02, 0x01, 0x51, 0x09, 0x06],
    }
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
        "control_change" => {
            if let Some(controller) = event.controller {
                if event.value.unwrap_or(0) >= 64 {
                    if !device.activity.active_controls.contains(&controller) {
                        device.activity.active_controls.push(controller);
                        device.activity.active_controls.sort_unstable();
                    }
                } else {
                    device
                        .activity
                        .active_controls
                        .retain(|active| *active != controller);
                }
                if controller == 64 {
                    device.activity.sustain = event.value.unwrap_or(0) >= 64;
                }
            }
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

    #[test]
    fn parses_alsa_sequencer_peers() {
        let clients = parse_seq_clients(
            r#"Client  24 : "Launchpad" [Kernel Legacy]
  Port   0 : "Launchpad MIDI 1" (RWeX) [In/Out]
    Connecting To: 128:0, 140:2
    Connected From: 150:1
Client 128 : "studio-control-novation.launchpad-original" [User Legacy]
  Port   0 : "observe" (-We-) [Out]
    Connected From: 24:0
"#,
        );
        assert_eq!(clients.len(), 2);
        assert_eq!(clients[0].id, 24);
        assert_eq!(clients[0].ports[&0].connecting_to, vec![(128, 0), (140, 2)]);
        assert_eq!(clients[0].ports[&0].connected_from, vec![(150, 1)]);
    }

    #[test]
    fn launchpad_mapping_and_palette_match_original_protocol() {
        assert_eq!(launchpad_velocity(0), 12);
        assert_eq!(launchpad_velocity(1), 60);
        assert_eq!(launchpad_velocity(2), 15);
        assert_eq!(launchpad_velocity(3), 63);
        assert_eq!(launchpad_velocity(4), 62);
        assert_eq!(3 * 16 + 5, 53);
    }

    #[test]
    fn activity_tracks_launchpad_top_buttons() {
        let state = Arc::new(Mutex::new(initial_snapshot()));
        apply_midi_event(
            &state,
            IncomingMidi {
                device_id: "novation.launchpad-original".to_string(),
                bytes: vec![0xb0, 104, 127],
            },
        );
        assert_eq!(
            state.lock().unwrap().devices["novation.launchpad-original"]
                .activity
                .active_controls,
            vec![104]
        );
        apply_midi_event(
            &state,
            IncomingMidi {
                device_id: "novation.launchpad-original".to_string(),
                bytes: vec![0xb0, 104, 0],
            },
        );
        assert!(
            state.lock().unwrap().devices["novation.launchpad-original"]
                .activity
                .active_controls
                .is_empty()
        );
    }
}
