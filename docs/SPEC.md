# Omarchy Studio Control — product and technical specification

Status: Launchpad managed-canvas milestone

## Product intent

Studio Control is a general hardware surface inside Omarchy. It is not a
combined status lamp for one fixed pair of devices. A single bar entry opens a
workspace containing a distinct, recognizable panel for every registered
piece of hardware. Each panel owns its appearance, status, capabilities, and
actions.

The initial hardware happens to be a Novation Launchpad, Behringer UMC404HD,
and Casio USB-MIDI keyboard. None receives special architectural status:
future devices are added as modules.

## Experience principles

1. **One device, one identity.** Every device has its own connection indicator,
   panel, controls, and error state.
2. **Recognizable hardware.** Panels should resemble the physical object
   closely enough that control placement feels familiar.
3. **Honest control.** A physical-only knob may be drawn for orientation, but
   the UI must not imply that software can read or move it.
4. **Hot-plug first.** Device identity is stable USB vendor/product metadata,
   never an ALSA card number that changes after reconnecting.
5. **Extensible by module.** Adding hardware should not require rewriting the
   studio shell.
6. **Local by default.** Core MIDI/audio control works without an internet
   service. Home Assistant is an optional device/integration module later.
7. **Observation is harmless.** A newly discovered device is never written to,
   rerouted, or claimed exclusively without an explicit mode change.

## Device access modes

Every module declares a default and supported access modes. The daemon applies
the mode per physical device rather than globally.

| Mode | Behaviour |
|---|---|
| `released` | Studio Control opens no port and receives no events. |
| `observe` | Non-exclusive input subscription and read-only system state. No output or routing changes. This is the default. |
| `managed` | Enables deliberate output and control actions while remaining cooperative with other applications. |
| `exclusive` | Optional future mode that may disconnect or block other clients. It must always require confirmation. |

MIDI observation uses ALSA sequencer subscriptions, allowing DAWs and other
clients to receive the same events concurrently. Raw MIDI ownership is not used
for the passive milestone.

## MVP scope

### Bar widget

- Studio glyph.
- One small status dot per registered device, not a combined health light.
- Clicking opens the studio surface.

### Studio surface

- Header with registered and connected counts.
- Horizontal device canvas that can grow and scroll as modules are added.
- Realistic Launchpad, UMC404HD, and Casio keyboard faces.
- Live USB connected/disconnected state refreshed without restarting shell.
- Launchpad pads mirror live button presses and provide local color preview in
  observe mode.
- Launchpad managed mode paints the physical LED grid, cycles colors from
  hardware or screen presses, and scrolls user-entered text.
- ALSA sequencer input consumers and LED output producers are shown separately;
  managed LED mode refuses a known output conflict.
- UMC404HD physical controls are visibly marked as hardware-only.

### Explicitly deferred

- Persistent Launchpad mappings/profiles and exclusive device takeover.
- PipeWire routing, level metering, and recording.
- Persistent profiles and mapping editor.
- Home Assistant API bridge.
- Per-device settings UI.

## Device module contract

Each module lives at `plugin/devices/<module>/` and contains:

```text
device.json
DevicePanel.qml
```

`device.json` version 1:

```json
{
  "schemaVersion": 1,
  "id": "vendor.device",
  "name": "Human name",
  "kind": "midi-controller",
  "panel": "DevicePanel.qml",
  "match": {
    "usb": [
      { "vendorId": "1234", "productId": "abcd" }
    ]
  },
  "access": {
    "defaultMode": "observe",
    "supportedModes": ["released", "observe", "managed"]
  },
  "capabilities": ["midi.input", "midi.output"]
}
```

The discovery helper scans module manifests and Linux USB sysfs, then returns
one state record per registered module. The shell uses the panel path from the
record, so adding a module adds a panel without modifying `Panel.qml`.

Every `DevicePanel.qml` root provides:

```qml
property var deviceState
signal actionRequested(string action, var payload)
```

The shell routes `actionRequested` through `studioctl` to the user daemon. Each
command produces a structured success/error reply. Observe-mode visual previews
remain local and never write hardware.

## Runtime architecture

### Visual MVP

```text
Omarchy shell / Quickshell
  └─ Studio Control bar widget
       ├─ device-status process
       │    ├─ device manifests
       │    └─ /sys/bus/usb/devices
       └─ dynamic QML device panels
```

### Control milestone

```text
Omarchy shell plugin ── JSON IPC ── studio-control user daemon
                                      ├─ device registry/hot-plug
                                      ├─ ALSA MIDI
                                      ├─ PipeWire/ALSA audio
                                      ├─ mapping profiles
                                      └─ optional Home Assistant client
```

The QML shell remains presentation-only. Long-running MIDI and audio work must
not run in the shared Omarchy shell process.

### Observe and managed-device flow

```text
ALSA MIDI source ── non-exclusive subscription ── Rust daemon
                                                    │
                                      Unix socket JSON stream
                                                    │
                                             Omarchy panel

Omarchy panel ── acknowledged command ── Rust daemon ── MIDI LED output
                                                    (managed mode only)
```

The daemon does not open MIDI output at startup, alter ALSA subscriptions
belonging to other clients, change PipeWire metadata, or open UMC audio capture
streams. Launchpad output opens only in managed mode and closes on release.

## Initial control roadmap

1. **Visual shell:** module discovery, live connectivity, realistic faces. ✓
2. **Passive daemon and IPC:** cooperative MIDI observation, live UI activity,
   logging, and reconnect behaviour. ✓
3. **Output Analyzer:** observe the real default PipeWire sink and publish
   reduced meters, spectrum, waveform, and loudness frames.
4. **REAPER observe adapter:** publish project, transport, record-arm, and
   recording-safety state through the shared adapter contract.
5. **Launchpad managed mode:** explicit release/control mode, LED renderer,
   button mirroring, and text canvas. ✓ (profiles remain)
6. **UMC404HD:** default input/output, digital mute/volume, meters, recording.
7. **System EQ:** explicit managed PipeWire filter-chain/LV2 mode with
   reversible routing.
8. **Assignments:** bind Launchpad controls to daemon actions and reflect state
   on both the hardware LEDs and QML face.
9. **Additional modules:** generic MIDI, Home Assistant, lighting, and DAWs.

Cross-domain adapters and the normalized event envelope are specified in
[`INTEGRATIONS.md`](INTEGRATIONS.md).

## Non-goals and hardware truth

UMC404HD gain, mix, phones, main-output, pad, line/instrument, monitor, and
phantom-power controls are hardware-side and do not publish their positions to
the host. Studio Control can show their physical placement for orientation but
cannot truthfully mirror or actuate them.

The original Launchpad is a MIDI controller with red/green/amber LED feedback;
it is not an audio device and its pads are not velocity-sensitive.

The connected Casio identifies itself generically as `CASIO USB-MIDI`
(`07cf:6803`), so its module represents the observed USB-MIDI keyboard rather
than claiming a specific retail keyboard model.
