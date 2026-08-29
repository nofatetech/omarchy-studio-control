import QtQuick

Item {
  id: root

  property color ledColor: "#52e881"
  property bool lit: true
  property real diameter: 7

  implicitWidth: diameter
  implicitHeight: diameter

  Rectangle {
    anchors.centerIn: parent
    width: root.diameter
    height: width
    radius: width / 2
    color: root.lit ? root.ledColor : "#202528"
    border.width: 1
    border.color: root.lit ? Qt.lighter(root.ledColor, 1.25) : "#454b4e"

    Rectangle {
      visible: root.lit
      anchors.centerIn: parent
      width: parent.width + 5
      height: width
      radius: width / 2
      color: Qt.rgba(root.ledColor.r, root.ledColor.g, root.ledColor.b, 0.16)
    }
  }
}

