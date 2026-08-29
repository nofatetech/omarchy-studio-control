import QtQuick
import "../../components" as Studio

Studio.DeviceFrame {
  id: root

  signal actionRequested(string action, var payload)

  title: deviceState && deviceState.name ? deviceState.name : "Novation Launchpad"
  subtitle: managed ? "MIDI controller · LED canvas managed" : "MIDI controller · observe only"
  accentColor: "#f39b36"
  implicitWidth: 430

  property var previewColors: ({})
  readonly property var activity: deviceState && deviceState.activity ? deviceState.activity : ({})
  readonly property var activeNotes: Array.isArray(activity.activeNotes) ? activity.activeNotes : []
  readonly property var activeControls: Array.isArray(activity.activeControls) ? activity.activeControls : []
  readonly property var lastEvent: activity.lastEvent || ({})
  readonly property var surface: deviceState && deviceState.surface ? deviceState.surface : ({})
  readonly property var surfaceCells: Array.isArray(surface.cells) ? surface.cells : []
  readonly property var peers: deviceState && deviceState.midiPeers ? deviceState.midiPeers : ({})
  readonly property var inputConsumers: Array.isArray(peers.inputConsumers) ? peers.inputConsumers : []
  readonly property var outputProducers: Array.isArray(peers.outputProducers) ? peers.outputProducers : []
  readonly property bool managed: deviceState && deviceState.accessMode === "managed"

  function padKey(row, column) { return row + ":" + column }
  function colorHex(index) {
    if (index === 1) return "#53dc78"
    if (index === 2) return "#ef5057"
    if (index === 3) return "#f1a13a"
    if (index === 4) return "#e8dc54"
    return "#252b2e"
  }
  function padValue(row, column) {
    var index = row * 8 + column
    if (managed && index < surfaceCells.length) return Number(surfaceCells[index] || 0)
    return Number(previewColors[padKey(row, column)] || 0)
  }
  function padColor(row, column) {
    var midiNote = row * 16 + column
    var base = colorHex(padValue(row, column))
    return activeNotes.indexOf(midiNote) >= 0 ? Qt.lighter(base === "#252b2e" ? "#4a8db7" : base, 1.3) : base
  }
  function cyclePad(row, column) {
    var nextValue = (padValue(row, column) + 1) % 5
    if (managed) {
      actionRequested("set_pad", { "x": column, "y": row, "color": nextValue })
    } else {
      var key = padKey(row, column)
      var next = Object.assign({}, previewColors)
      next[key] = nextValue
      previewColors = next
    }
  }
  function peerNames(list) {
    var names = []
    for (var i = 0; i < list.length; i++) names.push(list[i].name || (list[i].clientId + ":" + list[i].portId))
    return names.join(", ")
  }

  Item {
    width: parent.width
    implicitHeight: 510

    Rectangle {
      id: hardware
      anchors.horizontalCenter: parent.horizontalCenter
      width: 360
      height: 390
      radius: 10
      color: "#171b1d"
      border.width: 2
      border.color: managed ? "#6d522b" : "#303638"

      Rectangle {
        anchors.fill: parent
        anchors.margins: 7
        radius: 7
        color: "transparent"
        border.width: 1
        border.color: "#0b0d0e"
      }

      Text {
        anchors.left: parent.left
        anchors.leftMargin: 17
        anchors.top: parent.top
        anchors.topMargin: 13
        text: "novation"
        color: "#e9edef"
        font.pixelSize: 11
        font.weight: Font.DemiBold
        font.italic: true
      }

      Text {
        anchors.right: parent.right
        anchors.rightMargin: 18
        anchors.top: parent.top
        anchors.topMargin: 14
        text: "LAUNCHPAD"
        color: "#7f898e"
        font.pixelSize: 8
        font.weight: Font.DemiBold
        font.letterSpacing: 1.4
      }

      Row {
        anchors.horizontalCenter: grid.horizontalCenter
        y: 40
        spacing: 7

        Repeater {
          model: ["↑", "↓", "←", "→", "session", "user 1", "user 2", "mixer"]

          Column {
            required property var modelData
            required property int index
            spacing: 3

            Rectangle {
              anchors.horizontalCenter: parent.horizontalCenter
              width: 29
              height: 20
              radius: 10
              color: root.activeControls.indexOf(104 + parent.index) >= 0 ? "#f1a13a" : "#242a2c"
              border.width: 1
              border.color: root.activeControls.indexOf(104 + parent.index) >= 0 ? "#ffe0a0" : "#4d565a"
            }

            Text {
              anchors.horizontalCenter: parent.horizontalCenter
              text: modelData
              color: "#788287"
              font.pixelSize: modelData.length > 2 ? 6 : 8
            }
          }
        }
      }

      Grid {
        id: grid
        x: 28
        y: 88
        rows: 8
        columns: 8
        spacing: 7

        Repeater {
          model: 64

          Rectangle {
            required property int index
            property int padRow: Math.floor(index / 8)
            property int padColumn: index % 8
            width: 31
            height: 31
            radius: 4
            color: root.padColor(padRow, padColumn)
            border.width: 1
            border.color: root.padValue(padRow, padColumn) === 0 ? "#4b5357" : Qt.lighter(color, 1.25)

            Rectangle {
              anchors.fill: parent
              anchors.margins: 3
              radius: 2
              color: "transparent"
              border.width: 1
              border.color: Qt.rgba(1, 1, 1, 0.06)
            }

            TapHandler { onTapped: root.cyclePad(parent.padRow, parent.padColumn) }
          }
        }
      }

      Column {
        anchors.left: grid.right
        anchors.leftMargin: 8
        anchors.top: grid.top
        spacing: 7

        Repeater {
          model: ["vol", "pan", "snd A", "snd B", "stop", "trk on", "solo", "arm"]

          Rectangle {
            required property var modelData
            required property int index
            width: 28
            height: 31
            radius: 15
            color: root.activeNotes.indexOf(index * 16 + 8) >= 0 ? "#ef5057" : "#242a2c"
            border.width: 1
            border.color: root.activeNotes.indexOf(index * 16 + 8) >= 0 ? "#ff989c" : "#4b5357"

            Text {
              anchors.centerIn: parent
              text: modelData
              color: "#758085"
              font.pixelSize: 5
              font.weight: Font.DemiBold
            }
          }
        }
      }

      Text {
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 12
        anchors.horizontalCenter: parent.horizontalCenter
        text: managed
          ? (surface.animation || "MANAGED · tap hardware or screen pads to cycle colors")
          : (lastEvent.kind ? "LIVE " + String(lastEvent.kind).replace("_", " ").toUpperCase() : "OBSERVE · screen-pad colors are a local preview")
        color: managed ? "#d9ae6c" : "#667075"
        font.pixelSize: 7
      }
    }

    Row {
      id: controls
      anchors.top: hardware.bottom
      anchors.topMargin: 13
      anchors.horizontalCenter: parent.horizontalCenter
      spacing: 10

      Studio.HardwareButton {
        label: root.managed ? "RELEASE" : "MANAGE LEDs"
        interactive: deviceState && deviceState.connected
        checked: root.managed
        accentColor: "#f1a13a"
        onPressed: root.actionRequested("set_mode", { "mode": root.managed ? "observe" : "managed" })
      }

      Studio.HardwareButton {
        label: "CLEAR"
        interactive: root.managed
        onPressed: root.actionRequested("clear", {})
      }

      Rectangle {
        width: 118
        height: 27
        radius: 4
        color: "#181d1f"
        border.width: 1
        border.color: textInput.activeFocus ? "#9a7138" : "#41494d"

        TextInput {
          id: textInput
          anchors.fill: parent
          anchors.margins: 6
          text: "STUDIO"
          color: "#e1e5e6"
          selectionColor: "#715026"
          font.pixelSize: 10
          maximumLength: 32
          verticalAlignment: TextInput.AlignVCenter
        }
      }

      Studio.HardwareButton {
        label: "SCROLL"
        interactive: root.managed
        checked: !!surface.animation
        accentColor: "#53dc78"
        onPressed: root.actionRequested("scroll", { "text": textInput.text, "color": 3 })
      }

      Studio.HardwareButton {
        label: "STOP"
        interactive: root.managed && !!surface.animation
        onPressed: root.actionRequested("stop", {})
      }
    }

    Text {
      anchors.top: controls.bottom
      anchors.topMargin: 9
      anchors.horizontalCenter: parent.horizontalCenter
      width: parent.width - 20
      text: outputProducers.length > 0
        ? "LED CONFLICT · " + root.peerNames(outputProducers)
        : (inputConsumers.length > 0
          ? "BUTTON INPUT SHARED WITH · " + root.peerNames(inputConsumers)
          : "NO OTHER ALSA MIDI CLIENTS · raw-MIDI users are not visible here")
      color: outputProducers.length > 0 ? "#ef777c" : "#667075"
      font.pixelSize: 7
      font.letterSpacing: 0.35
      horizontalAlignment: Text.AlignHCenter
      elide: Text.ElideRight
    }
  }
}
