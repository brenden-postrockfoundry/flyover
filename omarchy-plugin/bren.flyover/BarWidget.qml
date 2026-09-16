import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "Model.js" as Model

// Ambient "aircraft nearby" count for the bar. Click launches the flyover
// TUI scope in a new terminal. Deliberately thin: this widget owns no
// scope-drawing logic of its own — that all lives in the flyover binary
// (~/flight-radar).
//
// No hyprctl/focus-existing-window logic: this Hyprland build replaced the
// classic string dispatchers (`hyprctl dispatch focuswindow class:...`) with
// a Lua-table API (hl.dsp.window.*) that doesn't obviously expose "focus by
// class" the same way, and getting that exactly right needs more digging
// than this widget is worth blocking on. launchProc.running doubles as a
// crude de-dupe instead: Quickshell won't restart a Process that's already
// running, so clicking again while the scope is still open is a no-op
// rather than a second window — not as good as true focus-by-class, but
// correct and simple.
BarWidget {
  id: root
  moduleName: "bren.flyover"

  readonly property string weatherLocationPath: Quickshell.env("HOME") + "/.local/state/omarchy/settings/weather.json"
  readonly property string scopeAppId: "flyover-scope"
  readonly property string scopeDir: Quickshell.env("HOME") + "/flight-radar"
  readonly property string scopeBinary: scopeDir + "/target/release/flyover"

  property real latitude: NaN
  property real longitude: NaN
  readonly property bool hasLocation: !isNaN(latitude) && !isNaN(longitude)

  property int aircraftCount: -1
  // Plain ASCII on purpose: Qt's Text item doesn't clip overflow, so if an
  // emoji glyph rendered wider than the font-metrics width Qt used to size
  // the button, the visible pill would be wider than its actual clickable
  // hit-area — clicks on the overflowing part would land on nothing.
  readonly property string displayText: !hasLocation
    ? "AC ?"
    : (aircraftCount < 0 ? "AC .." : ("AC " + aircraftCount))

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  FileView {
    id: locationFile
    path: root.weatherLocationPath
    watchChanges: true
    onLoaded: {
      var loc = Model.parseLocationFile(text())
      root.latitude = loc.latitude
      root.longitude = loc.longitude
    }
    onLoadFailed: {
      root.latitude = NaN
      root.longitude = NaN
    }
  }

  function refresh() {
    if (root.hasLocation && !fetchProc.running) fetchProc.running = true
  }

  Process {
    id: fetchProc
    command: ["curl", "-fsS", "--max-time", "5",
      "https://api.adsb.lol/v2/point/" + root.latitude + "/" + root.longitude + "/100"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        root.aircraftCount = Model.countAircraft(String(text || ""))
      }
    }
  }

  // Ambient signal only — polls far less often than the scope app itself,
  // which fetches every ~10s while actually open.
  Timer {
    interval: 20000
    running: root.hasLocation
    repeat: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }

  function openScope() {
    console.log("bren.flyover: openScope() pressed")
    launchProc.running = true
  }

  // Direct argv invocation — no shell involved, so no quoting to get wrong.
  // Absolute path since Quickshell's Process isn't guaranteed to inherit an
  // interactive shell's PATH.
  Process {
    id: launchProc
    command: ["/usr/bin/foot", "-a", root.scopeAppId, "-T", "flyover", "-H", "-D", root.scopeDir, root.scopeBinary]
    stderr: StdioCollector {
      onStreamFinished: if (text) console.warn("bren.flyover launch stderr: " + text)
    }
    onExited: function(exitCode) {
      console.log("bren.flyover: scope process exited with code " + exitCode)
    }
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.displayText
    onPressed: function(b) { root.openScope() }
  }
}
