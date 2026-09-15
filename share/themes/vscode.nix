{ colors, filecolors }:

let
    c = colors;
    hex = h: "#" + h;

    bg = hex c.bg;
    dbg = hex c.dbg;
    lbg = hex c.lbg;
    fg = hex c.fg;
    textOnPrimary = hex c.c15;

    c0  = hex c.c0;
    c1  = hex c.c1;
    c2  = hex c.c2;
    c3  = hex c.c3;
    c4  = hex c.c4;
    c5  = hex c.c5;
    c6  = hex c.c6;
    c7  = hex c.c7;
    c8  = hex c.c8;
    c9  = hex c.c9;
    c10 = hex c.c10;
    c11 = hex c.c11;
    c12 = hex c.c12;
    c13 = hex c.c13;
    c14 = hex c.c14;
    c15 = hex c.c15;

    primary = hex c.primary;
    primaryBright = hex c.primaryBright;
in

{
    packageJson = builtins.toJSON {
        name = "neptunia-theme";
        displayName = "Neptunia";
        description = "Neptunia colorscheme, generated from the nixdots theme";
        version = "1.0.0";
        publisher = "nix";
        engines.vscode = "^1.75.0";
        categories = [ "Themes" ];
        contributes.themes = [
            {
                label = "Neptunia";
                uiTheme = "vs-dark";
                path = "./themes/neptunia-color-theme.json";
            }
        ];
    };

    themeJson = builtins.toJSON {
        name = "Neptunia";
        type = "dark";

        colors = {
            focusBorder = primary;
            foreground = fg;
            descriptionForeground = c8;
            errorForeground = c1;
            warningForeground = c11;
            infoForeground = c4;
            hintForeground = c6;

            "panel.background" = bg;
            "panel.border" = lbg;
            "panelTitle.activeBorder" = primary;
            "panelTitle.activeForeground" = fg;
            "panelTitle.inactiveForeground" = c8;
            "panelSection.border" = lbg;

            "commandCenter.background" = bg;
            "commandCenter.activeBackground" = c0;
            "commandCenter.border" = lbg;
            "commandCenter.foreground" = fg;
            "commandCenter.activeForeground" = fg;

            "titleBar.activeBackground" = bg;
            "titleBar.activeForeground" = fg;
            "titleBar.inactiveBackground" = dbg;
            "titleBar.inactiveForeground" = c8;
            "menubar.selectionBackground" = lbg;
            "menubar.selectionForeground" = textOnPrimary;

            "activityBar.background" = dbg;
            "activityBar.foreground" = c8;
            "activityBar.inactiveForeground" = c8;
            "activityBar.activeBorder" = primary;
            "activityBarBadge.background" = primary;
            "activityBarBadge.foreground" = textOnPrimary;

            "sideBar.background" = dbg;
            "sideBar.foreground" = fg;
            "sideBar.border" = lbg;
            "sideBarSectionHeader.background" = bg;
            "sideBarSectionHeader.foreground" = fg;
            "sideBarTitle.foreground" = fg;

            "list.activeSelectionBackground" = lbg;
            "list.activeSelectionForeground" = textOnPrimary;
            "list.inactiveSelectionBackground" = lbg;
            "list.inactiveSelectionForeground" = fg;
            "list.hoverBackground" = c0;
            "list.hoverForeground" = fg;
            "list.focusBackground" = c0;
            "list.focusOutline" = primary;
            "list.highlightForeground" = primaryBright;
            "listFilterWidget.background" = bg;
            "listFilterWidget.outline" = primary;

            "editor.background" = dbg;
            "editor.foreground" = fg;
            "editorLineNumber.foreground" = c8;
            "editorLineNumber.activeForeground" = c15;
            "editorCursor.foreground" = primaryBright;
            "editor.selectionBackground" = lbg;
            "editor.selectionForeground" = textOnPrimary;
            "editor.inactiveSelectionBackground" = lbg;
            "editor.wordHighlightBackground" = lbg;
            "editor.wordHighlightStrongBackground" = primary;
            "editor.lineHighlightBackground" = lbg;
            "editor.lineHighlightBorder" = lbg;
            "editorWhitespace.foreground" = lbg;
            "editorIndentGuide.background1" = lbg;
            "editorIndentGuide.background2" = c8;
            "editorIndentGuide.activeBackground1" = primary;
            "editorIndentGuide.activeBackground2" = primary;
            "editorBracketMatch.background" = c0;
            "editorBracketMatch.border" = c1;
            "editorGutter.background" = dbg;
            "editorError.foreground" = c1;
            "editorWarning.foreground" = c11;
            "editorInfo.foreground" = c4;
            "editorWidget.background" = bg;
            "editorWidget.foreground" = fg;
            "editorWidget.border" = lbg;
            "editorWidget.resizeBorder" = primary;
            "editorSuggestWidget.background" = bg;
            "editorSuggestWidget.border" = lbg;
            "editorSuggestWidget.foreground" = fg;
            "editorSuggestWidget.selectedBackground" = lbg;
            "editorSuggestWidget.selectedForeground" = textOnPrimary;
            "editorSuggestWidget.highlightForeground" = primaryBright;
            "editorHoverWidget.background" = bg;
            "editorHoverWidget.foreground" = fg;
            "editorHoverWidget.border" = lbg;

            "editorGroupHeader.tabsBackground" = bg;
            "editorGroupHeader.border" = lbg;
            "tab.border" = lbg;
            "tab.activeBackground" = bg;
            "tab.activeForeground" = fg;
            "tab.activeBorderTop" = primary;
            "tab.inactiveBackground" = bg;
            "tab.inactiveForeground" = c8;
            "tab.hoverBackground" = c0;
            "tab.hoverForeground" = fg;
            "tab.unfocusedActiveForeground" = c8;
            "tab.unfocusedActiveBorderTop" = lbg;
            "tab.unfocusedInactiveForeground" = c8;

            "statusBar.background" = bg;
            "statusBar.foreground" = c8;
            "statusBar.border" = lbg;
            "statusBar.noFolderBackground" = dbg;
            "statusBar.debuggingBackground" = c1;
            "statusBar.debuggingForeground" = textOnPrimary;
            "statusBarItem.hoverBackground" = c0;
            "statusBarItem.remoteBackground" = primary;
            "statusBarItem.remoteForeground" = textOnPrimary;
            "statusBarItem.prominentBackground" = primary;
            "statusBarItem.prominentForeground" = textOnPrimary;

            "terminal.background" = dbg;
            "terminal.foreground" = fg;
            "terminalCursor.background" = dbg;
            "terminalCursor.foreground" = c7;
            "terminal.selectionBackground" = primary;
            "terminal.selectionForeground" = textOnPrimary;
            "terminal.ansiBlack" = c0;
            "terminal.ansiRed" = c1;
            "terminal.ansiGreen" = c2;
            "terminal.ansiYellow" = c3;
            "terminal.ansiBlue" = c4;
            "terminal.ansiMagenta" = c5;
            "terminal.ansiCyan" = c6;
            "terminal.ansiWhite" = c7;
            "terminal.ansiBrightBlack" = c8;
            "terminal.ansiBrightRed" = c9;
            "terminal.ansiBrightGreen" = c10;
            "terminal.ansiBrightYellow" = c11;
            "terminal.ansiBrightBlue" = c12;
            "terminal.ansiBrightMagenta" = c13;
            "terminal.ansiBrightCyan" = c14;
            "terminal.ansiBrightWhite" = c15;

            "scrollbarSlider.background" = bg;
            "scrollbarSlider.hoverBackground" = c8;
            "scrollbarSlider.activeBackground" = c8;

            "input.background" = bg;
            "input.foreground" = fg;
            "input.border" = c8;
            "input.placeholderForeground" = c8;
            "inputOption.activeBorder" = primary;
            "dropdown.background" = bg;
            "dropdown.border" = c8;
            "dropdown.foreground" = fg;
            "checkbox.background" = bg;
            "checkbox.border" = c8;
            "checkbox.foreground" = fg;
            "button.background" = primary;
            "button.foreground" = textOnPrimary;
            "button.hoverBackground" = primaryBright;
            "button.secondaryBackground" = lbg;
            "button.secondaryForeground" = fg;
            "button.secondaryHoverBackground" = c0;

            "badge.background" = primary;
            "badge.foreground" = textOnPrimary;
            "breadcrumb.foreground" = c8;
            "breadcrumb.focusForeground" = fg;
            "breadcrumb.activeSelectionForeground" = primaryBright;
            "notificationCenter.background" = bg;
            "notificationCenter.border" = lbg;
            "notificationToast.background" = bg;
            "notification.background" = bg;
            "notification.border" = lbg;

            "minimap.background" = dbg;
            "minimap.selectionHighlight" = primary;
            "minimap.findMatchHighlight" = lbg;
            "editorOverviewRuler.border" = lbg;
            "editorOverviewRuler.commonContentForeground" = c3;
            "editorOverviewRuler.addedForeground" = c2;
            "editorOverviewRuler.deletedForeground" = c1;
            "editorOverviewRuler.modifiedForeground" = c11;
            "editorOverviewRuler.errorForeground" = c1;
            "editorOverviewRuler.warningForeground" = c11;
            "editorOverviewRuler.infoForeground" = c4;

            "progressBar.background" = primary;
            "pickerGroup.foreground" = primaryBright;
            "pickerGroup.border" = lbg;
            "quickInput.background" = dbg;
            "quickInput.foreground" = fg;
            "quickInputList.focusBackground" = lbg;
            "quickInputList.focusForeground" = textOnPrimary;
            "keybindingLabel.background" = c0;
            "keybindingLabel.foreground" = c7;
            "gitDecoration.addedResourceForeground" = c2;
            "gitDecoration.modifiedResourceForeground" = c11;
            "gitDecoration.deletedResourceForeground" = c1;
            "gitDecoration.untrackedResourceForeground" = c6;
            "gitDecoration.ignoredResourceForeground" = c8;
            "gitDecoration.conflictingResourceForeground" = c5;
            "problemsErrorIcon.foreground" = c1;
            "problemsWarningIcon.foreground" = c11;
            "problemsInfoIcon.foreground" = c4;
            "debugIcon.breakpointForeground" = c1;
            "charts.foreground" = c7;
            "charts.lines" = c8;
            "charts.red" = c1;
            "charts.green" = c2;
            "charts.yellow" = c11;
            "charts.blue" = c4;
            "charts.magenta" = c5;
            "charts.cyan" = c6;
        };

        tokenColors = [
            {
                scope = [
                    "comment"
                    "punctuation.definition.comment"
                ];
                settings.foreground = c8;
                settings.fontStyle = "italic";
            }
            {
                scope = "comment.block.documentation";
                settings.foreground = c8;
                settings.fontStyle = "italic";
            }
            {
                scope = [
                    "constant.character"
                    "constant.other"
                    "constant.language"
                ];
                settings.foreground = c3;
            }
            {
                scope = "constant.numeric";
                settings.foreground = c3;
            }
            {
                scope = "constant.character.escape";
                settings.foreground = c9;
            }
            {
                scope = "entity.name.function";
                settings.foreground = c9;
            }
            {
                scope = "entity.name.type";
                settings.foreground = c14;
            }
            {
                scope = "entity.other.inherited-class";
                settings.foreground = c6;
            }
            {
                scope = [
                    "entity.name.tag"
                    "entity.other.tag"
                ];
                settings.foreground = c4;
            }
            {
                scope = "entity.other.attribute-name";
                settings.foreground = c3;
            }
            {
                scope = [
                    "keyword"
                    "keyword.control"
                ];
                settings.foreground = c12;
            }
            {
                scope = "keyword.operator";
                settings.foreground = c12;
            }
            {
                scope = [
                    "storage.type"
                    "storage.modifier"
                ];
                settings.foreground = c6;
            }
            {
                scope = [
                    "string"
                    "string.regexp"
                    "constant.other.regexp"
                ];
                settings.foreground = c10;
            }
            {
                scope = [
                    "support.function"
                    "support.macro"
                ];
                settings.foreground = c9;
            }
            {
                scope = "support.constant";
                settings.foreground = c3;
            }
            {
                scope = "support.type";
                settings.foreground = c14;
            }
            {
                scope = [
                    "variable"
                    "variable.parameter"
                    "variable.other"
                ];
                settings.foreground = c7;
            }
            {
                scope = [
                    "variable.language"
                    "variable.other.constant"
                ];
                settings.foreground = c13;
            }
            {
                scope = [
                    "invalid"
                    "invalid.illegal"
                ];
                settings.foreground = c1;
            }
            {
                scope = [
                    "punctuation.separator"
                    "punctuation.definition"
                ];
                settings.foreground = c8;
            }
            {
                scope = "meta.tag";
                settings.foreground = c6;
            }
            {
                scope = "token.error-token";
                settings.foreground = c1;
            }
            {
                scope = "token.warn-token";
                settings.foreground = c11;
            }
            {
                scope = "token.info-token";
                settings.foreground = c4;
            }
            {
                scope = "token.debug-token";
                settings.foreground = c5;
            }
            {
                scope = "markup.heading";
                settings.foreground = c4;
                settings.fontStyle = "bold";
            }
            {
                scope = "markup.bold";
                settings.fontStyle = "bold";
            }
            {
                scope = "markup.italic";
                settings.fontStyle = "italic";
            }
            {
                scope = [
                    "markup.quote"
                    "markup.list"
                ];
                settings.foreground = c2;
            }
            {
                scope = "markup.inline.raw";
                settings.foreground = c3;
            }
            {
                scope = "markup.link";
                settings.foreground = c4;
                settings.fontStyle = "underline";
            }
            {
                scope = "markup.raw";
                settings.foreground = c10;
            }
        ];
    };
}