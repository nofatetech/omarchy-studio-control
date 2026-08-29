import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

Panel {
  id: root

  moduleName: "io.github.nofatetech.studio-control"
  ipcTarget: "io.github.nofatetech.studio-control"

  property var settings: ({})
  property var deviceSnapshot: ({ "devices": [], "connectedCount": 0, "registeredCount": 0 })
  property var observerDevices: ({})
  property int activityRevision: 0
  property bool observerOnline: false
  property string observerError: ""
  property string discoveryError: ""

  readonly property var devices: Array.isArray(deviceSnapshot.devices) ? deviceSnapshot.devices : []
  readonly property int connectedCount: Number(deviceSnapshot.connectedCount || 0)
  readonly property int registeredCount: Number(deviceSnapshot.registeredCount || devices.length)
  readonly property int refreshIntervalMs: Math.max(1000, Number(setting("refreshIntervalMs", 2500)))
  readonly property bool showDisconnected: setting("showDisconnected", true) !== false
  readonly property string pluginDirectory: localPath(Qt.resolvedUrl("."))
  readonly property string statusCommand: pluginDirectory + "/scripts/device-status"
  readonly property string studioctlCommand: Quickshell.env("HOME") + "/.local/bin/studioctl"
  readonly property color foreground: bar ? bar.foreground : Color.foreground
  readonly property color dim: Qt.rgba(foreground.r, foreground.g, foreground.b, 0.55)
  readonly property color accent: Color.accent
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family

  implicitWidth: barButton.implicitWidth
  implicitHeight: barButton.implicitHeight

  function setting(name, fallback) {
    if (!settings || settings[name] === undefined || settings[name] === null) return fallback
    return settings[name]
  }

  function localPath(url) {
    var path = String(url || "")
    if (path.slice(0, 7) === "file://") path = path.slice(7)
    while (path.length > 1 && path.slice(-1) === "/") path = path.slice(0, -1)
    return decodeURIComponent(path)
  }

  function visibleDevices() {
    if (showDisconnected) return devices
    var visible = []
    for (var i = 0; i < devices.length; i++) {
      if (devices[i] && devices[i].connected) visible.push(devices[i])
    }
    return visible
  }

  function applySnapshot(output) {
    var text = String(output || "").trim()
    if (text === "") {
      discoveryError = "Device discovery returned no data"
      return
    }
    try {
      var parsed = JSON.parse(text)
      if (!parsed || !Array.isArray(parsed.devices)) throw new Error("missing devices array")
      deviceSnapshot = parsed
      discoveryError = ""
    } catch (error) {
      discoveryError = "Could not read device registry: " + error
    }
  }

  function refreshDevices() {
    if (!statusProcess.running) statusProcess.running = true
  }

  function enrichedDeviceState(device, revision) {
    if (!device) return ({})
    var result = Object.assign({}, device)
    var observed = observerDevices && device.id ? observerDevices[device.id] : null
    if (observed) {
      result.activity = observed.activity || ({})
      result.accessMode = observed.accessMode || "observe"
      result.midiConnected = observed.midiConnected === true
      result.midiPort = observed.midiPort || ""
      result.observerOnline = observerOnline
    }
    return result
  }

  function applyObserverMessage(line) {
    var text = String(line || "").trim()
    if (text === "") return
    try {
      var message = JSON.parse(text)
      if (message.type === "snapshot" && message.snapshot && message.snapshot.devices) {
        observerDevices = message.snapshot.devices
        observerOnline = true
        observerError = ""
        activityRevision++
        return
      }
      if (message.type === "midi_event" && message.event && message.event.deviceId) {
        var next = Object.assign({}, observerDevices)
        var current = Object.assign({}, next[message.event.deviceId] || ({}))
        current.activity = message.activity || ({})
        current.midiConnected = true
        current.accessMode = "observe"
        next[message.event.deviceId] = current
        observerDevices = next
        observerOnline = true
        observerError = ""
        activityRevision++
      }
    } catch (error) {
      observerError = "Observer data error: " + error
    }
  }

  function connectionColor(device) {
    if (device && device.error) return "#e6a441"
    return device && device.connected ? "#52e881" : "#ec5d62"
  }

  Component.onCompleted: refreshDevices()
  onOpenedChanged: if (opened) refreshDevices()

  Timer {
    interval: root.refreshIntervalMs
    running: true
    repeat: true
    onTriggered: root.refreshDevices()
  }

  Process {
    id: statusProcess
    command: [root.statusCommand]
    running: false

    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: root.applySnapshot(text)
    }

    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: if (text.trim() !== "") root.discoveryError = text.trim()
    }
  }

  Process {
    id: observerProcess
    command: [root.studioctlCommand, "watch"]
    running: true

    stdout: SplitParser {
      onRead: function(line) { root.applyObserverMessage(line) }
    }

    stderr: SplitParser {
      onRead: function(line) {
        var message = String(line || "").trim()
        if (message !== "") root.observerError = message
      }
    }

    onExited: function(exitCode) {
      root.observerOnline = false
      if (root.observerError === "") root.observerError = "Observer offline (" + exitCode + ")"
      observerRetry.restart()
    }
  }

  Timer {
    id: observerRetry
    interval: 2000
    repeat: false
    onTriggered: if (!observerProcess.running) observerProcess.running = true
  }

  Item {
    id: barButton
    implicitWidth: iconButton.implicitWidth + Math.max(0, deviceDots.implicitWidth - Style.space(8))
    implicitHeight: iconButton.implicitHeight

    BarIconButton {
      id: iconButton
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      bar: root.bar
      text: "󰓃"
      active: root.opened
      onPressed: function(buttonCode) { root.toggle() }
    }

    Row {
      id: deviceDots
      anchors.right: parent.right
      anchors.rightMargin: 1
      anchors.bottom: parent.bottom
      anchors.bottomMargin: 3
      spacing: 2

      Repeater {
        model: root.devices

        Rectangle {
          required property var modelData
          width: 5
          height: 5
          radius: width / 2
          color: root.connectionColor(modelData)
          border.width: 1
          border.color: Qt.rgba(0, 0, 0, 0.45)
        }
      }
    }
  }

  KeyboardPanel {
    id: studioPanel
    anchorItem: barButton
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: fittedContentWidth(Math.min(Style.space(1040), Math.max(Style.space(560), deviceRow.implicitWidth + Style.space(32))))
    contentHeight: fittedContentHeight(studioContent.implicitHeight, Style.space(660))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent

      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(text) {
        if (text === "r" || text === "R") root.refreshDevices()
      }

      Column {
        id: studioContent
        width: parent.width
        spacing: Style.space(13)

        Item {
          width: parent.width
          implicitHeight: 48

          Column {
            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2

            Text {
              text: "Studio Control"
              color: root.foreground
              font.family: root.fontFamily
              font.pixelSize: Style.font.title
              font.weight: Font.DemiBold
            }

            Text {
              text: root.connectedCount + " connected · " + root.registeredCount + " modules · observer " + (root.observerOnline ? "live" : "offline")
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
            }
          }

          Row {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            spacing: Style.space(8)

            Repeater {
              model: root.devices

              Rectangle {
                required property var modelData
                implicitWidth: statusRow.implicitWidth + 18
                implicitHeight: 28
                radius: 14
                color: modelData && modelData.connected ? "#13291c" : "#2b1719"
                border.width: 1
                border.color: modelData && modelData.connected ? "#28583a" : "#5c2a2d"

                Row {
                  id: statusRow
                  anchors.centerIn: parent
                  spacing: 6

                  Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 6
                    height: width
                    radius: width / 2
                    color: root.connectionColor(modelData)
                  }

                  Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData.shortName || modelData.name || "Device"
                    color: root.foreground
                    font.family: root.fontFamily
                    font.pixelSize: Style.font.caption
                  }
                }
              }
            }

            Button {
              text: "Refresh"
              bordered: true
              foreground: root.foreground
              fontFamily: root.fontFamily
              fontSize: Style.font.bodySmall
              onClicked: root.refreshDevices()
            }
          }
        }

        BorderSurface {
          visible: root.discoveryError !== ""
          width: parent.width
          implicitHeight: errorText.implicitHeight + Style.space(20)
          color: Qt.rgba(0.88, 0.32, 0.35, 0.08)
          borderSpec: Border.flat(Qt.rgba(0.88, 0.32, 0.35, 0.32), 1)
          radius: Style.cornerRadius

          Text {
            id: errorText
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: Style.space(10)
            text: root.discoveryError
            color: "#e98b8e"
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            wrapMode: Text.WordWrap
          }
        }

        Flickable {
          id: deviceCanvas
          width: parent.width
          implicitHeight: Math.min(deviceRow.implicitHeight, Style.space(540))
          contentWidth: deviceRow.implicitWidth
          contentHeight: deviceRow.implicitHeight
          clip: true
          boundsBehavior: Flickable.StopAtBounds
          flickableDirection: Flickable.HorizontalFlick

          Row {
            id: deviceRow
            spacing: Style.space(12)

            Repeater {
              model: root.visibleDevices()

              Loader {
                required property var modelData
                property var enrichedState: root.enrichedDeviceState(modelData, root.activityRevision)
                source: "file://" + modelData.panel

                onLoaded: {
                  if (!item) return
                  item.deviceState = enrichedState
                }

                onEnrichedStateChanged: if (item) item.deviceState = enrichedState
              }
            }
          }
        }

        Text {
          width: parent.width
          text: "R  refresh devices   ·   drag horizontally for more hardware   ·   Esc  close"
          color: root.dim
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          horizontalAlignment: Text.AlignHCenter
        }
      }
    }
  }
}
