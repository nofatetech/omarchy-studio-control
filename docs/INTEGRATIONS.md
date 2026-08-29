# Studio Control integration architecture

Studio Control is an observe-first view of the local studio, not a replacement
for PipeWire, a DAW, Home Assistant, or a lighting console. It normalizes state
from those systems and makes deliberate control available only when an entity
is placed in a managed mode.

## Responsibility boundary

```text
hardware / application / service
             │ native protocol
             ▼
       small domain adapter
             │ normalized Studio events
             ▼
       studio-control-daemon
        ├─ current state
        ├─ optional history
        └─ access-mode policy
             │ local JSON-lines IPC
             ▼
        Omarchy shell panel
```

The daemon owns normalized state and access policy. Adapters speak the native
protocol of their domain. QML renders reduced state and never performs audio,
MIDI, lighting, or DAW work directly.

High-rate inputs must be reduced before IPC. For example, an audio adapter sends
meter and spectrum bins at a UI-friendly cadence instead of forwarding PCM
samples into the shell process.

## Shared event contract

Adapters should publish a versioned event envelope independent of transport:

```json
{
  "schemaVersion": 1,
  "type": "state",
  "source": "adapter.reaper",
  "entityId": "app.reaper.transport",
  "sequence": 42,
  "timestampMs": 1788039969775,
  "mode": "observe",
  "payload": {
    "playing": true,
    "recording": false,
    "positionSeconds": 18.4
  }
}
```

Rules:

- `entityId` is stable and namespaced; ALSA card and PipeWire object numbers are
  runtime details, not identities.
- Every snapshot and event carries a schema version.
- Commands include a request ID and produce an acknowledged result or error.
- `observe` adapters publish state but expose no mutating commands.
- Managed commands declare their reversibility and previous value where
  practical.
- Unknown fields are ignored so minor schema additions remain compatible.
- Secrets and raw media never appear in events or logs.

The current Unix socket and JSON-lines stream remain the local UI transport.
Domain protocols are not tunneled through QML.

## Standards to use at the edges

| Domain | Preferred standard or API | Studio Control role |
|---|---|---|
| Linux audio/video graph | PipeWire with WirePlumber metadata/policy | Observe nodes, defaults, formats, links, latency, and health. Use managed mode for routing changes. |
| Audio DSP | PipeWire filter-chain and LV2 | Build an optional system-output EQ without inventing a plugin format. |
| Loudness | EBU R128 / ITU-R BS.1770 semantics | Momentary, short-term, integrated loudness, and true-peak displays. |
| MIDI | ALSA sequencer today; MIDI 1.0 semantics; MIDI 2.0 UMP/MIDI-CI when hardware supports it | Cooperative observation, capability discovery, and explicit managed output. |
| DAWs and performance software | Open Sound Control (OSC) | One adapter per application-specific OSC namespace. |
| Desktop media players | MPRIS over the session D-Bus | Player identity, metadata, playback, position, and optional managed transport. |
| Lighting | sACN (ANSI E1.31) and Art-Net; DMX512 at the fixture edge | Observe universes/nodes; transmit only in managed mode. |
| Smart home | Home Assistant WebSocket API for state streams and REST/service calls for actions; MQTT where it already exists | Treat Home Assistant entities as external modules instead of reimplementing device integrations. |
| Long-term telemetry | Optional OpenTelemetry/Prometheus-style exporter | Export slow health metrics without putting a telemetry stack in the real-time path. |

Standards define transport and domain semantics, but there is no single
professional-studio schema that covers MIDI devices, DAWs, lighting, system
audio, and room sensors. The small versioned envelope above is the intentional
local normalization layer.

## MIDI ownership and Launchpad policy

ALSA sequencer subscriptions are directional. Studio Control reports external
clients receiving events from a hardware port as `inputConsumers`, and clients
sending events to it as `outputProducers`. Multiple input consumers can safely
observe the same Launchpad button events. Competing output producers can fight
over its LEDs, so managed mode is rejected when one is visible.

This is cooperative ownership, not a lock: applications using Linux raw MIDI
directly do not appear in the ALSA sequencer graph. Studio Control never
disconnects peers in managed mode. A future exclusive mode must identify and
confirm affected clients before changing subscriptions.

The original Launchpad uses MIDI 1.0 note messages for the 8×8 grid and side
buttons, CC 104–111 for its top row, and a four-color red/green LED palette.
Animation sends only changed cells at a bounded cadence to stay comfortably
below the device's documented update ceiling.

## Output Analyzer and System EQ

These are separate modules because their safety properties differ.

### Output Analyzer — `observe`

- Follow the WirePlumber default audio sink and its monitor stream.
- Show the real sink name, sample rate, channel layout, quantum/latency, volume,
  mute, and routing changes.
- Compute stereo peak/RMS, EBU-style loudness, clipping, spectrum bands,
  oscilloscope samples, and stereo correlation in the daemon.
- Send only reduced numerical frames to QML at roughly 20–30 Hz.
- Offer visual modes inspired by classic players: 10/31-band spectrum,
  waveform, vectorscope, peak hold, and restrained persistence trails.

### System EQ — `managed`

- Ten familiar bands: 31, 62, 125, 250, 500 Hz and 1, 2, 4, 8, 16 kHz.
- Preamp/headroom, bypass, presets, clipping warning, and an obvious active
  indicator.
- Implement DSP as PipeWire biquads/filter-chain or an LV2 plugin.
- Create a named virtual sink and ask before making or routing it as default.
- Restore the previous default and links when disabled or if setup fails.
- Never imply that this changes REAPER's master FX; DAW processing remains in
  the DAW to prevent accidental double-EQ.

## REAPER integration

The existing `reaper-omarchy-integration` repository should continue to own
theme generation and the Omarchy `theme-set` hook. That job does not belong in
the Studio Control daemon.

A separate, optional REAPER adapter can use REAPER's supported OSC control
surface interface over loopback. Its observe profile should publish:

- running/available state and active project name;
- play, pause, record, repeat, position, tempo, and time signature;
- track count, selected track, record-arm state, and mute/solo summaries;
- render/recording state and useful meter summaries when available.

Managed mode can later expose transport and selected actions. The adapter and
theme hook remain independently installable; they share the event contract,
entity/capability names, and access-mode vocabulary rather than source code.

## Useful observability modules

1. **Audio graph health:** defaults, routes, sample rate, quantum, latency,
   xruns, clipping, and active capture/playback clients.
2. **Recording safety:** REAPER recording/armed state, current project, free
   recording-disk space, sustained write rate, and accidental input loss.
3. **MIDI health:** connected endpoints, event rate, stuck notes, sustain,
   channel activity, MIDI clock/MTC, and detected chords.
4. **Hardware health:** USB reconnects/errors, device identity, power state,
   and access ownership.
5. **Room/environment:** lighting scenes, temperature/humidity, ambient noise,
   occupancy, and power/UPS state through Home Assistant where available.
6. **Session context:** an explicit session timer and markers for rehearsing,
   recording, streaming, or idle—local by default, not surveillance.
7. **Privacy/safety:** conspicuous indicators whenever a microphone, camera,
   recording path, network broadcast, DMX output, or managed device mode is
   active.

## Suggested sequence

1. Casio full keybed and chord state.
2. Real PipeWire Output Analyzer in observe mode.
3. REAPER OSC adapter in observe mode.
4. Session/recording-safety summary.
5. Managed PipeWire EQ with reversible routing.
6. Home Assistant and sACN/Art-Net adapters as hardware arrives.
