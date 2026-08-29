import QtQuick

Rectangle {
  id: root

  property var deviceState: ({})
  property string title: deviceState && deviceState.name ? deviceState.name : "Studio device"
  property string subtitle: deviceState && deviceState.kind ? deviceState.kind : "hardware"
  property color accentColor: "#6ed49b"
  default property alias body: bodyContainer.data

  implicitWidth: 410
  implicitHeight: header.height + bodyContainer.implicitHeight + footer.height + 26
  radius: 12
  color: "#111517"
  border.width: 1
  border.color: deviceState && deviceState.connected
    ? Qt.rgba(accentColor.r, accentColor.g, accentColor.b, 0.42)
    : "#343a3d"

  Rectangle {
    anchors.fill: parent
    anchors.margins: 1
    radius: root.radius - 1
    color: "transparent"
    border.width: 1
    border.color: Qt.rgba(1, 1, 1, 0.025)
  }

  Item {
    id: header
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.margins: 14
    height: 36

    Column {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      spacing: 1

      Text {
        text: root.title
        color: "#f1f4f5"
        font.pixelSize: 14
        font.weight: Font.DemiBold
      }

      Text {
        text: String(root.subtitle).replace("-", " ").toUpperCase()
        color: "#737d82"
        font.pixelSize: 8
        font.letterSpacing: 1.1
      }
    }

    Row {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      spacing: 7

      Rectangle {
        anchors.verticalCenter: parent.verticalCenter
        width: 8
        height: width
        radius: width / 2
        color: root.deviceState && root.deviceState.connected ? "#52e881" : "#ec5d62"

        Rectangle {
          visible: root.deviceState && root.deviceState.connected
          anchors.centerIn: parent
          width: parent.width + 7
          height: width
          radius: width / 2
          color: Qt.rgba(0.32, 0.91, 0.51, 0.13)
        }
      }

      Text {
        text: root.deviceState && root.deviceState.connected ? "CONNECTED" : "DISCONNECTED"
        color: root.deviceState && root.deviceState.connected ? "#8fe9ae" : "#e98b8e"
        font.pixelSize: 8
        font.weight: Font.DemiBold
        font.letterSpacing: 0.7
      }
    }
  }

  Item {
    id: bodyContainer
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: header.bottom
    anchors.leftMargin: 14
    anchors.rightMargin: 14
    implicitHeight: childrenRect.height
  }

  Item {
    id: footer
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: bodyContainer.bottom
    anchors.leftMargin: 14
    anchors.rightMargin: 14
    anchors.topMargin: 10
    height: 23

    Text {
      anchors.left: parent.left
      anchors.verticalCenter: parent.verticalCenter
      text: root.deviceState && root.deviceState.usb
        ? String(root.deviceState.usb.vendorId) + ":" + String(root.deviceState.usb.productId)
        : "USB device unavailable"
      color: "#5f696d"
      font.family: "monospace"
      font.pixelSize: 8
    }

    Text {
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      text: "MODULE 01"
      color: "#424a4e"
      font.pixelSize: 7
      font.letterSpacing: 0.8
    }
  }
}

