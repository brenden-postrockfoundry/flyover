import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "Model.js" as Model

// Ambient "aircraft nearby" count for the bar. Click launches the flyover
// TUI scope in a new terminal, or focuses it if one is already running.
// Deliberately thin: this widget owns no scope-drawing logic of its own —
// that all lives in the flyover binary (~/flight-radar).
BarWidget {
  id: root
  moduleName: "bren.flight-radar-pill"

  readonly property string weatherLocationPath: Quickshell.env("HOME") + "/.local/state/omarchy/settings/weather.json"
  readonly property string scopeAppId: "flyover-scope"
  readonly property string scopeDir: Quickshell.env("HOME") + "/flight-radar"
  readonly property string scopeBinary: scopeDir + "/target/release/flyover"

  property real latitude: NaN
  property real longitude: NaN
  readonly property bool hasLocation: !isNaN(latitude) && !isNaN(longitude)

  property int aircraftCount: -1
  readonly property string displayText: !hasLocation
    ? "✈ ?"
    : (aircraftCount < 0 ? "✈ …" : ("✈ " + aircraftCount))

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
    launchProc.running = true
  }

  // No IPC with the scope app in v1 (see project-ideas.md) — this just
  // focuses an existing window by app-id, or launches a new one.
  Process {
    id: launchProc
    command: ["bash", "-c",
      "if hyprctl clients -j | jq -e '.[] | select(.class == \"" + root.scopeAppId + "\")' >/dev/null 2>&1; then " +
      "hyprctl dispatch focuswindow 'class:^(" + root.scopeAppId + ")$'; " +
      "else " +
      "foot -a " + root.scopeAppId + " -T flyover -H -D '" + root.scopeDir + "' '" + root.scopeBinary + "' & disown; " +
      "fi"
    ]
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: root.displayText
    onPressed: function(b) { root.openScope() }
  }
}
