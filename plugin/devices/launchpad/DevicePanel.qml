import QtQuick
import "../../components" as Studio

Studio.DeviceFrame {
  id: root

  signal actionRequested(string action, var payload)

  title: deviceState && deviceState.name ? deviceState.name : "Novation Launchpad"
  subtitle: "MIDI controller · original"
  accentColor: "#f39b36"
  implicitWidth: 416

  property var padColors: ({})

  function padKey(row, column) { return row + ":" + column }

  function padColor(row, column) {
    var colorIndex = Number(padColors[padKey(row, column)] || 0)
    if (colorIndex === 1) return "#53dc78"
    if (colorIndex === 2) return "#ef5057"
    if (colorIndex === 3) return "#f1a13a"
    return "#252b2e"
  }

  function cyclePad(row, column) {
    var key = padKey(row, column)
    var next = Object.assign({}, padColors)
    next[key] = (Number(next[key] || 0) + 1) % 4
    padColors = next
    actionRequested("preview.pad", { "row": row, "column": column, "color": next[key] })
  }

  Item {
    width: parent.width
    implicitHeight: 420

    Rectangle {
      id: hardware
      anchors.horizontalCenter: parent.horizontalCenter
      width: 360
      height: 390
      radius: 10
      color: "#171b1d"
      border.width: 2
      border.color: "#303638"

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
        id: topButtons
        anchors.horizontalCenter: grid.horizontalCenter
        y: 40
        spacing: 7

        Repeater {
          model: ["↑", "↓", "←", "→", "session", "user 1", "user 2", "mixer"]

          Column {
            required property var modelData
            spacing: 3

            Rectangle {
              anchors.horizontalCenter: parent.horizontalCenter
              width: 29
              height: 20
              radius: 10
              color: "#242a2c"
              border.width: 1
              border.color: "#4d565a"
            }

            Text {
              anchors.horizontalCenter: parent.horizontalCenter
              text: modelData
              color: "#6f797e"
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
            border.color: color === "#252b2e" ? "#4b5357" : Qt.lighter(color, 1.25)

            Rectangle {
              anchors.fill: parent
              anchors.margins: 3
              radius: 2
              color: "transparent"
              border.width: 1
              border.color: Qt.rgba(1, 1, 1, 0.06)
            }

            TapHandler {
              onTapped: root.cyclePad(parent.padRow, parent.padColumn)
            }
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
            width: 28
            height: 31
            radius: 15
            color: "#242a2c"
            border.width: 1
            border.color: "#4b5357"

            Text {
              anchors.centerIn: parent
              text: modelData
              color: "#687277"
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
        text: "Click pads to preview the original red · green · amber palette"
        color: "#667075"
        font.pixelSize: 7
      }
    }
  }
}
