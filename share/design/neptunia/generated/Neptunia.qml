// Generado desde share/design/neptunia. Úsalo como singleton (qmldir: singleton Neptunia 1.0 Neptunia.qml).
pragma Singleton
import QtQuick

QtObject {
    property string mode: "light" // "light" | "dark" | "dark-tint"
    readonly property color accent: mode === "dark" ? "#2f6fe4" : mode === "dark-tint" ? "#2f6fe4" : "#2f6fe4"
    readonly property color accentHover: mode === "dark" ? "#2560cf" : mode === "dark-tint" ? "#2560cf" : "#2560cf"
    readonly property color accentSoft: mode === "dark" ? "#1f3358" : mode === "dark-tint" ? "#1f3358" : "#dbe7fd"
    readonly property color accentStrong: mode === "dark" ? "#6aa2ff" : mode === "dark-tint" ? "#6aa2ff" : "#1f53bd"
    readonly property color bgControl: mode === "dark" ? "#3a3a3a" : mode === "dark-tint" ? "#38393d" : "#e3e8ef"
    readonly property color bgHover: mode === "dark" ? "#444444" : mode === "dark-tint" ? "#424448" : "#dde3eb"
    readonly property color bgRaised: mode === "dark" ? "#2b2b2b" : mode === "dark-tint" ? "#2a2b2e" : "#ffffff"
    readonly property color bgSunken: mode === "dark" ? "#161616" : mode === "dark-tint" ? "#151617" : "#e7ebf1"
    readonly property color bgSurface: mode === "dark" ? "#262626" : mode === "dark-tint" ? "#252628" : "#ffffff"
    readonly property color bgWindow: mode === "dark" ? "#1e1e1e" : mode === "dark-tint" ? "#1d1e20" : "#f2f4f7"
    readonly property color cyan: mode === "dark" ? "#4fb6e3" : mode === "dark-tint" ? "#4fb6e3" : "#0a6f9a"
    readonly property color danger: mode === "dark" ? "#ff6b6b" : mode === "dark-tint" ? "#ff6b6b" : "#cc2f35"
    readonly property color dangerFill: mode === "dark" ? "#d6363b" : mode === "dark-tint" ? "#d6363b" : "#d6363b"
    readonly property color info: mode === "dark" ? "#b18cff" : mode === "dark-tint" ? "#b18cff" : "#7a49c9"
    readonly property color ink: mode === "dark" ? "#dcdcdc" : mode === "dark-tint" ? "#dadcdf" : "#1b2330"
    readonly property color inkFaint: mode === "dark" ? "#6b6b6b" : mode === "dark-tint" ? "#696b70" : "#8a96a8"
    readonly property color inkMuted: mode === "dark" ? "#9b9b9b" : mode === "dark-tint" ? "#999ca3" : "#55657e"
    readonly property color inkStrong: mode === "dark" ? "#f5f5f5" : mode === "dark-tint" ? "#f4f5f6" : "#0d1219"
    readonly property color line: mode === "dark" ? "#333333" : mode === "dark-tint" ? "#323336" : "#d6dce5"
    readonly property color lineStrong: mode === "dark" ? "#747474" : mode === "dark-tint" ? "#73767c" : "#848fa1"
    readonly property color onAccent: mode === "dark" ? "#ffffff" : mode === "dark-tint" ? "#ffffff" : "#ffffff"
    readonly property color onDanger: mode === "dark" ? "#ffffff" : mode === "dark-tint" ? "#ffffff" : "#ffffff"
    readonly property color scrim: mode === "dark" ? "#80000000" : mode === "dark-tint" ? "#80000000" : "#470d1219"
    readonly property color success: mode === "dark" ? "#5fcf7c" : mode === "dark-tint" ? "#5fcf7c" : "#1a7f37"
    readonly property color sysBlue: mode === "dark" ? "#2f6fe4" : mode === "dark-tint" ? "#2f6fe4" : "#2f6fe4"
    readonly property color sysCyan: mode === "dark" ? "#0f86b8" : mode === "dark-tint" ? "#0f86b8" : "#0f86b8"
    readonly property color sysGray: mode === "dark" ? "#7d828c" : mode === "dark-tint" ? "#7d828c" : "#7d828c"
    readonly property color sysGreen: mode === "dark" ? "#28a046" : mode === "dark-tint" ? "#28a046" : "#28a046"
    readonly property color sysOrange: mode === "dark" ? "#e8780e" : mode === "dark-tint" ? "#e8780e" : "#e8780e"
    readonly property color sysPurple: mode === "dark" ? "#8e5bd8" : mode === "dark-tint" ? "#8e5bd8" : "#8e5bd8"
    readonly property color sysRed: mode === "dark" ? "#d6363b" : mode === "dark-tint" ? "#d6363b" : "#d6363b"
    readonly property color trafficClose: mode === "dark" ? "#e0625d" : mode === "dark-tint" ? "#e0625d" : "#e0625d"
    readonly property color trafficMin: mode === "dark" ? "#d9a63a" : mode === "dark-tint" ? "#d9a63a" : "#d9a63a"
    readonly property color trafficZoom: mode === "dark" ? "#4fae5f" : mode === "dark-tint" ? "#4fae5f" : "#4fae5f"
    readonly property color warning: mode === "dark" ? "#f0a43a" : mode === "dark-tint" ? "#f0a43a" : "#a15c00"
    readonly property color windowInner: mode === "dark" ? "#24ffffff" : mode === "dark-tint" ? "#24ffffff" : "#00ffffff"
    readonly property color windowOuter: mode === "dark" ? "#e6000000" : mode === "dark-tint" ? "#e6000000" : "#1f0d1219"
    readonly property int radiusButton: 8
    readonly property int radiusLg: 16
    readonly property int radiusMd: 12
    readonly property int radiusPill: 999
    readonly property int radiusSm: 8
    readonly property int radiusXl: 16
    readonly property int radiusXs: 4
    readonly property int space1: 4
    readonly property int space2: 8
    readonly property int space3: 12
    readonly property int space4: 16
    readonly property int space6: 24
    readonly property int space8: 32
    readonly property int controlLg: 38
    readonly property int controlMd: 30
    readonly property int controlSm: 24
    readonly property int sidebarW: 220
    readonly property int titlebarH: 44
    readonly property int durBase: 220
    readonly property int durFast: 120
    readonly property int durSlow: 420
    readonly property string fontSans: "Geist"
    readonly property string fontMono: "SF Mono Ligaturized"
}
