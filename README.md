# Omarchy Studio Control

An extensible Omarchy shell surface for hardware used in a studio: audio
interfaces, MIDI controllers, lighting controllers, and whatever gets plugged
in next.

The project starts with two device modules:

- Novation Launchpad (`1235:000e`)
- Behringer UMC404HD (`1397:0509`)

The first milestone is intentionally visual. Clicking the Studio Control bar
widget opens a device surface containing a realistic face for every registered
device. Connection state is read from Linux USB sysfs; hardware actions are
added incrementally behind the same device-module contract.

## Current milestone

- Manifest-driven device modules loaded dynamically by the shell
- One independent connection state per registered device
- Live USB discovery without fixed ALSA card numbers
- Realistic Launchpad and UMC404HD device faces
- Interactive Launchpad color preview
- Horizontally scalable studio surface for additional hardware

MIDI input/output, Launchpad LED feedback, PipeWire controls, recording, and
Home Assistant integration are planned but not part of this visual MVP. See
[the product and technical specification](docs/SPEC.md) for the module contract
and roadmap.

## Requirements

- Omarchy with the Quickshell-based shell/plugin system
- Linux USB sysfs mounted at `/sys/bus/usb/devices`
- Python 3 for manifest-driven device discovery

## Repository layout

```text
plugin/
  manifest.json                 Omarchy shell plugin manifest
  Panel.qml                     bar widget and multi-device studio surface
  components/                   reusable visual controls
  devices/<device-id>/
    device.json                 discovery and capability metadata
    DevicePanel.qml             device-specific face
  scripts/device-status         manifest-driven USB discovery
docs/
  SPEC.md                       product and architecture specification
scripts/
  install-dev                   install the working tree as a user plugin
  uninstall-dev                 remove the development installation
```

## Development preview

Run:

```bash
./scripts/install-dev
```

The installer symlinks `plugin/` into the user Omarchy plugin directory and
adds `io.github.nofatetech.studio-control` to the right side of the bar. Source
edits then hot-reload through Omarchy shell.

No files under `/usr/share/omarchy` are modified.

To remove the development symlink:

```bash
./scripts/uninstall-dev
```

## Verify discovery

Run the discovery helper directly:

```bash
./plugin/scripts/device-status | python3 -m json.tool
```

Every directory under `plugin/devices/` containing a valid `device.json` and
`DevicePanel.qml` becomes a registered device module. Connected hardware is
matched using stable identifiers declared by the module manifest.
