# Omarchy Studio Control — product and technical specification

Status: MVP visual shell

## Product intent

Studio Control is a general hardware surface inside Omarchy. It is not a
combined status lamp for one fixed pair of devices. A single bar entry opens a
workspace containing a distinct, recognizable panel for every registered
piece of hardware. Each panel owns its appearance, status, capabilities, and
actions.

The initial hardware happens to be a Novation Launchpad and Behringer
UMC404HD. Neither receives special architectural status: future devices are
added as modules.

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

## MVP scope

### Bar widget

- Studio glyph.
- One small status dot per registered device, not a combined health light.
- Clicking opens the studio surface.

### Studio surface

- Header with registered and connected counts.
- Horizontal device canvas that can grow and scroll as modules are added.
- Realistic Launchpad and UMC404HD faces.
- Live USB connected/disconnected state refreshed without restarting shell.
- Launchpad pads provide local visual interaction for immediate UI testing.
- UMC404HD physical controls are visibly marked as hardware-only.

### Explicitly deferred

- MIDI ownership, pad mappings, and LED output.
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

The shell will eventually route `actionRequested` to a user daemon. For the
visual MVP, device panels keep only harmless preview state locally.

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

## Initial control roadmap

1. **Visual shell:** module discovery, live connectivity, realistic faces.
2. **Daemon and IPC:** stable state/actions, logging, reconnect behavior.
3. **Launchpad:** input events, LED renderer, controller release mode, profiles.
4. **UMC404HD:** default input/output, digital mute/volume, meters, recording.
5. **Assignments:** bind Launchpad controls to daemon actions and reflect state
   on both the hardware LEDs and QML face.
6. **Additional modules:** generic MIDI, Home Assistant, lighting, and DAWs.

## Non-goals and hardware truth

UMC404HD gain, mix, phones, main-output, pad, line/instrument, monitor, and
phantom-power controls are hardware-side and do not publish their positions to
the host. Studio Control can show their physical placement for orientation but
cannot truthfully mirror or actuate them.

The original Launchpad is a MIDI controller with red/green/amber LED feedback;
it is not an audio device and its pads are not velocity-sensitive.

