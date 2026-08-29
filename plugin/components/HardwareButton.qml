import QtQuick

Item {
  id: root

  property string label: "PAD"
  property bool checked: false
  property bool interactive: false
  property color accentColor: "#f1a33a"

  signal pressed()

  implicitWidth: Math.max(34, labelText.implicitWidth + 12)
  implicitHeight: 27

  Rectangle {
    id: buttonFace
    anchors.horizontalCenter: parent.horizontalCenter
    width: 24
    height: 12
    radius: 2
    color: root.checked ? root.accentColor : "#24292b"
    border.width: 1
    border.color: root.checked ? Qt.lighter(root.accentColor, 1.25) : "#666d71"

    Rectangle {
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      height: 2
      radius: 1
      color: Qt.rgba(1, 1, 1, 0.12)
    }
  }

  Text {
    id: labelText
    anchors.top: buttonFace.bottom
    anchors.topMargin: 3
    anchors.horizontalCenter: parent.horizontalCenter
    text: root.label
    color: "#aeb6ba"
    font.pixelSize: 7
    font.weight: Font.DemiBold
    font.letterSpacing: 0.45
  }

  TapHandler {
    enabled: root.interactive
    onTapped: root.pressed()
  }
}

