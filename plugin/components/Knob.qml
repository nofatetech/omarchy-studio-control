import QtQuick

Item {
  id: root

  property string label: "GAIN"
  property real value: 0.48
  property real diameter: 34
  property color faceColor: "#171b1d"
  property color accentColor: "#e7ecef"
  property color labelColor: "#aeb6ba"

  implicitWidth: Math.max(diameter + 8, labelText.implicitWidth)
  implicitHeight: diameter + labelText.implicitHeight + 8

  Item {
    id: dial
    anchors.horizontalCenter: parent.horizontalCenter
    width: root.diameter
    height: width

    Rectangle {
      anchors.fill: parent
      radius: width / 2
      color: root.faceColor
      border.width: 2
      border.color: "#596064"

      Rectangle {
        anchors.centerIn: parent
        width: parent.width - 8
        height: width
        radius: width / 2
        color: "#242a2d"
        border.width: 1
        border.color: "#090b0c"
      }
    }

    Item {
      anchors.fill: parent
      rotation: -135 + Math.max(0, Math.min(1, root.value)) * 270

      Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        y: 5
        width: 2
        height: Math.max(6, root.diameter * 0.22)
        radius: 1
        color: root.accentColor
      }
    }
  }

  Text {
    id: labelText
    anchors.top: dial.bottom
    anchors.topMargin: 5
    anchors.horizontalCenter: parent.horizontalCenter
    text: root.label
    color: root.labelColor
    font.pixelSize: 8
    font.weight: Font.DemiBold
    font.letterSpacing: 0.7
  }
}

