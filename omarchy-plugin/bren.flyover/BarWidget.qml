import QtQuick
import qs.Ui

// TEMPORARY minimal diagnostic build. No WidgetButton, no FileView, no
// Process, no Timer — just a fixed-size clickable rectangle, to test
// whether the problem is in this widget's normal complexity or something
// more fundamental about how this bar renders/clicks third-party widgets.
BarWidget {
  id: root
  moduleName: "bren.flyover"

  width: 70
  height: bar ? bar.barSize : 30
  implicitWidth: width
  implicitHeight: height

  Rectangle {
    anchors.fill: parent
    color: mouseArea.pressed ? "lime" : "red"
    opacity: 0.6

    Text {
      anchors.centerIn: parent
      text: "TEST"
      color: "white"
    }
  }

  MouseArea {
    id: mouseArea
    anchors.fill: parent
    onClicked: console.log("bren.flyover: MINIMAL TEST CLICKED")
  }
}
