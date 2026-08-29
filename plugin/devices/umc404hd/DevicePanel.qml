import QtQuick
import "../../components" as Studio

Studio.DeviceFrame {
  id: root

  signal actionRequested(string action, var payload)

  title: deviceState && deviceState.name ? deviceState.name : "Behringer UMC404HD"
  subtitle: "4×4 USB audio / MIDI interface"
  accentColor: "#70b9e9"
  implicitWidth: 530

  readonly property var activity: deviceState && deviceState.activity ? deviceState.activity : ({})
  readonly property var lastEvent: activity.lastEvent || ({})

  Item {
    width: parent.width
    implicitHeight: 420

    Rectangle {
      id: rackFace
      anchors.horizontalCenter: parent.horizontalCenter
      width: 492
      height: 330
      radius: 7
      color: "#202527"
      border.width: 2
      border.color: "#444c50"

      Rectangle {
        anchors.fill: parent
        anchors.margins: 5
        radius: 4
        color: "transparent"
        border.width: 1
        border.color: "#0b0d0e"
      }

      Text {
        x: 18
        y: 13
        text: "U-PHORIA"
        color: "#dadfe1"
        font.pixelSize: 10
        font.weight: Font.Bold
        font.italic: true
      }

      Text {
        anchors.right: parent.right
        anchors.rightMargin: 18
        y: 12
        text: "UMC404HD"
        color: "#d8dddf"
        font.pixelSize: 13
        font.weight: Font.Bold
        font.letterSpacing: 0.5
      }

      Row {
        id: channels
        x: 16
        y: 47
        spacing: 5

        Repeater {
          model: 4

          Rectangle {
            required property int index
            width: 78
            height: 253
            radius: 3
            color: index % 2 === 0 ? "#1a1f21" : "#1d2224"
            border.width: 1
            border.color: "#343b3e"

            Text {
              anchors.top: parent.top
              anchors.topMargin: 7
              anchors.horizontalCenter: parent.horizontalCenter
              text: "INPUT " + (index + 1)
              color: "#bdc5c8"
              font.pixelSize: 8
              font.weight: Font.DemiBold
              font.letterSpacing: 0.6
            }

            Rectangle {
              anchors.top: parent.top
              anchors.topMargin: 26
              anchors.horizontalCenter: parent.horizontalCenter
              width: 42
              height: 42
              radius: width / 2
              color: "#0e1112"
              border.width: 3
              border.color: "#767e82"

              Rectangle {
                anchors.centerIn: parent
                width: 13
                height: 13
                radius: width / 2
                color: "#050606"
                border.width: 1
                border.color: "#3e4649"
              }

              Rectangle {
                anchors.centerIn: parent
                width: 4
                height: 22
                color: "#090b0c"
              }
            }

            Studio.Knob {
              anchors.top: parent.top
              anchors.topMargin: 83
              anchors.horizontalCenter: parent.horizontalCenter
              diameter: 39
              label: "GAIN " + (index + 1)
              value: [0.42, 0.55, 0.34, 0.48][index]
              accentColor: index < 2 ? "#63c789" : "#76b9e5"
            }

            Row {
              anchors.top: parent.top
              anchors.topMargin: 150
              anchors.horizontalCenter: parent.horizontalCenter
              spacing: 8

              Column {
                spacing: 3
                Studio.StatusLed { anchors.horizontalCenter: parent.horizontalCenter; ledColor: "#55dd75"; lit: index === 1; diameter: 6 }
                Text { text: "SIG"; color: "#657075"; font.pixelSize: 6 }
              }

              Column {
                spacing: 3
                Studio.StatusLed { anchors.horizontalCenter: parent.horizontalCenter; ledColor: "#ef5459"; lit: false; diameter: 6 }
                Text { text: "CLIP"; color: "#657075"; font.pixelSize: 6 }
              }
            }

            Row {
              anchors.bottom: parent.bottom
              anchors.bottomMargin: 14
              anchors.horizontalCenter: parent.horizontalCenter
              spacing: 1

              Studio.HardwareButton { label: "LINE/INST"; checked: index === 0 }
              Studio.HardwareButton { label: "PAD"; checked: false }
            }
          }
        }
      }

      Rectangle {
        anchors.left: channels.right
        anchors.leftMargin: 7
        anchors.top: channels.top
        width: 134
        height: channels.height
        radius: 3
        color: "#191e20"
        border.width: 1
        border.color: "#343b3e"

        Text {
          anchors.top: parent.top
          anchors.topMargin: 7
          anchors.horizontalCenter: parent.horizontalCenter
          text: "MONITORING"
          color: "#bdc5c8"
          font.pixelSize: 8
          font.weight: Font.DemiBold
          font.letterSpacing: 0.6
        }

        Row {
          anchors.top: parent.top
          anchors.topMargin: 35
          anchors.horizontalCenter: parent.horizontalCenter
          spacing: 8

          Studio.Knob { diameter: 38; label: "MIX"; value: 0.5; accentColor: "#f0a33b" }
          Studio.Knob { diameter: 38; label: "MAIN OUT"; value: 0.64; accentColor: "#70b9e9" }
        }

        Row {
          anchors.top: parent.top
          anchors.topMargin: 106
          anchors.horizontalCenter: parent.horizontalCenter
          spacing: 10

          Studio.HardwareButton { label: "STEREO/MONO"; checked: false }
          Studio.HardwareButton { label: "A/B"; checked: true; accentColor: "#70b9e9" }
        }

        Studio.Knob {
          anchors.top: parent.top
          anchors.topMargin: 153
          anchors.horizontalCenter: parent.horizontalCenter
          diameter: 45
          label: "PHONES"
          value: 0.43
          accentColor: "#d9dfe1"
        }

        Row {
          anchors.bottom: parent.bottom
          anchors.bottomMargin: 14
          anchors.horizontalCenter: parent.horizontalCenter
          spacing: 12

          Column {
            spacing: 4
            Studio.StatusLed { anchors.horizontalCenter: parent.horizontalCenter; ledColor: "#4fcf72"; lit: true; diameter: 7 }
            Text { text: "POWER"; color: "#657075"; font.pixelSize: 6 }
          }

          Column {
            spacing: 4
            Studio.StatusLed { anchors.horizontalCenter: parent.horizontalCenter; ledColor: "#ef5459"; lit: false; diameter: 7 }
            Text { text: "+48 V"; color: "#657075"; font.pixelSize: 6 }
          }
        }
      }
    }

    Rectangle {
      anchors.top: rackFace.bottom
      anchors.topMargin: 12
      anchors.horizontalCenter: parent.horizontalCenter
      width: rackFace.width
      height: 48
      radius: 7
      color: "#15191b"
      border.width: 1
      border.color: "#30373a"

      Text {
        anchors.left: parent.left
        anchors.leftMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        width: parent.width - 120
        text: root.lastEvent.kind
          ? "MIDI " + String(root.lastEvent.kind).replace("_", " ").toUpperCase() + " · " + Number(root.activity.eventCount || 0) + " observed events"
          : "Physical knob/switch positions are not exposed over USB. MIDI is observed without taking ownership."
        color: "#808a8f"
        font.pixelSize: 8
        wrapMode: Text.WordWrap
      }

      Row {
        anchors.right: parent.right
        anchors.rightMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6

        Studio.StatusLed {
          anchors.verticalCenter: parent.verticalCenter
          ledColor: "#70b9e9"
          lit: root.lastEvent.kind !== undefined
          diameter: 7
        }

        Text {
          anchors.verticalCenter: parent.verticalCenter
          text: "MIDI OBSERVE"
          color: "#64747c"
          font.pixelSize: 7
          font.weight: Font.DemiBold
        }
      }
    }
  }
}
