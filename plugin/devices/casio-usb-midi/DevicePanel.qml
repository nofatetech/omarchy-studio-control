import QtQuick
import "../../components" as Studio

Studio.DeviceFrame {
  id: root

  signal actionRequested(string action, var payload)

  title: deviceState && deviceState.name ? deviceState.name : "Casio USB-MIDI Keyboard"
  subtitle: "61-key velocity-sensitive MIDI keyboard"
  accentColor: "#62b6e7"
  implicitWidth: 760

  readonly property var activity: deviceState && deviceState.activity ? deviceState.activity : ({})
  readonly property var activeNotes: Array.isArray(activity.activeNotes) ? activity.activeNotes : []
  readonly property var lastEvent: activity.lastEvent || ({})
  readonly property bool sustain: activity.sustain === true
  readonly property string chord: activity.chord || ""
  readonly property string displayedChord: chord || activity.lastChord || "—"

  function whiteNote(index) {
    var offsets = [0, 2, 4, 5, 7, 9, 11]
    return 36 + Math.floor(index / 7) * 12 + offsets[index % 7]
  }

  function hasBlackAfter(note) {
    var pitch = note % 12
    return pitch !== 4 && pitch !== 11
  }

  function noteActive(note) {
    return activeNotes.indexOf(note) >= 0
  }

  function noteLabel(note) {
    var names = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"]
    if (note === undefined || note === null || Number(note) < 0) return "—"
    var value = Number(note)
    return names[value % 12] + (Math.floor(value / 12) - 1)
  }

  Item {
    width: parent.width
    implicitHeight: 315

    Rectangle {
      id: keyboardBody
      anchors.horizontalCenter: parent.horizontalCenter
      width: 720
      height: 262
      radius: 10
      color: "#202426"
      border.width: 2
      border.color: "#42494d"

      Rectangle {
        anchors.fill: parent
        anchors.margins: 6
        radius: 7
        color: "transparent"
        border.width: 1
        border.color: "#0c0e0f"
      }

      Text {
        x: 18
        y: 13
        text: "CASIO"
        color: "#edf1f2"
        font.pixelSize: 16
        font.weight: Font.Bold
        font.letterSpacing: 1.5
      }

      Text {
        anchors.right: parent.right
        anchors.rightMargin: 18
        y: 16
        text: "USB–MIDI"
        color: "#828c91"
        font.pixelSize: 9
        font.weight: Font.DemiBold
        font.letterSpacing: 1.2
      }

      Row {
        x: 19
        y: 48
        spacing: 8

        Rectangle {
          width: 184
          height: 48
          radius: 5
          color: "#141719"
          border.width: 1
          border.color: "#343a3d"

          Row {
            anchors.centerIn: parent
            spacing: 16

            Column {
              spacing: 2
              Text { text: root.noteLabel(root.lastEvent.note); color: "#79c8f1"; font.pixelSize: 18; font.weight: Font.DemiBold }
              Text { text: "LAST NOTE"; color: "#626c71"; font.pixelSize: 6; font.letterSpacing: 0.8 }
            }

            Column {
              spacing: 2
              Text { text: root.lastEvent.velocity !== undefined ? root.lastEvent.velocity : "—"; color: "#d7dddf"; font.pixelSize: 18; font.weight: Font.DemiBold }
              Text { text: "VELOCITY"; color: "#626c71"; font.pixelSize: 6; font.letterSpacing: 0.8 }
            }

            Column {
              spacing: 2
              Text { text: root.lastEvent.channel !== undefined ? Number(root.lastEvent.channel) + 1 : "—"; color: "#d7dddf"; font.pixelSize: 18; font.weight: Font.DemiBold }
              Text { text: "CHANNEL"; color: "#626c71"; font.pixelSize: 6; font.letterSpacing: 0.8 }
            }
          }
        }

        Rectangle {
          width: 74
          height: 48
          radius: 5
          color: root.sustain ? "#173124" : "#141719"
          border.width: 1
          border.color: root.sustain ? "#3f8b5d" : "#343a3d"

          Column {
            anchors.centerIn: parent
            spacing: 5
            Studio.StatusLed { anchors.horizontalCenter: parent.horizontalCenter; ledColor: "#52e881"; lit: root.sustain; diameter: 8 }
            Text { text: "SUSTAIN"; color: root.sustain ? "#8fe9ae" : "#626c71"; font.pixelSize: 7; font.weight: Font.DemiBold; font.letterSpacing: 0.7 }
          }
        }

        Rectangle {
          width: 128
          height: 48
          radius: 5
          color: root.chord !== "" ? "#182935" : "#141719"
          border.width: 1
          border.color: root.chord !== "" ? "#356781" : "#343a3d"

          Column {
            anchors.centerIn: parent
            spacing: 2
            Text {
              anchors.horizontalCenter: parent.horizontalCenter
              text: root.displayedChord
              color: root.chord !== "" ? "#8bd4f5" : "#8a969b"
              font.pixelSize: 17
              font.weight: Font.DemiBold
            }
            Text {
              anchors.horizontalCenter: parent.horizontalCenter
              text: root.chord !== "" ? "ACTIVE CHORD" : "LAST CHORD"
              color: "#626c71"
              font.pixelSize: 6
              font.letterSpacing: 0.8
            }
          }
        }

        Rectangle {
          width: 256
          height: 48
          radius: 5
          color: "#141719"
          border.width: 1
          border.color: "#343a3d"

          Row {
            anchors.centerIn: parent
            spacing: 12

            Studio.StatusLed { anchors.verticalCenter: parent.verticalCenter; ledColor: "#62b6e7"; lit: root.lastEvent.kind !== undefined; diameter: 8 }

            Column {
              spacing: 2
              Text { text: root.lastEvent.kind ? String(root.lastEvent.kind).replace("_", " ").toUpperCase() : "WAITING FOR MIDI"; color: "#cdd3d5"; font.pixelSize: 9; font.weight: Font.DemiBold }
              Text { text: "Observe mode · no MIDI output"; color: "#626c71"; font.pixelSize: 7 }
            }
          }
        }
      }

      Item {
        id: keybed
        x: 19
        y: 111
        width: 36 * 19
        height: 134

        Row {
          anchors.fill: parent
          spacing: 0

          Repeater {
            model: 36

            Rectangle {
              required property int index
              property int midiNote: root.whiteNote(index)
              width: 19
              height: keybed.height
              color: root.noteActive(midiNote) ? "#69bfe9" : "#e8ecec"
              border.width: 1
              border.color: "#25292b"

              Rectangle {
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 8
                anchors.horizontalCenter: parent.horizontalCenter
                width: 4
                height: 4
                radius: 2
                color: root.noteActive(parent.midiNote) ? "#d9f3ff" : "transparent"
              }

              Rectangle {
                visible: root.hasBlackAfter(parent.midiNote) && index < 35
                z: 2
                x: parent.width - 6
                y: 0
                width: 12
                height: 83
                radius: 0
                color: root.noteActive(parent.midiNote + 1) ? "#398fbd" : "#16191a"
                border.width: 1
                border.color: "#050606"
              }
            }
          }
        }
      }
    }

    Text {
      anchors.top: keyboardBody.bottom
      anchors.topMargin: 12
      anchors.horizontalCenter: parent.horizontalCenter
      text: "Detected as CASIO USB-MIDI 07cf:6803 · model name is not exposed over USB"
      color: "#687277"
      font.pixelSize: 8
    }
  }
}
