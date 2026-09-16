# islands — Safari-compact Firefox chrome, deployed via home-manager.
#
# This module:
#   1. injects the fx-autoconfig program-side config.js through the nixpkgs
#      Firefox wrapper's `extraPrefsFiles` (appended to <install>/lib/firefox/
#      mozilla.cfg, which autoconfig loads — the only path that works on Nix
#      because `general.config.filename` resolves against the read-only GRE
#      dir),
#   2. deploys the profile chrome tree (loader `utils/` + the islands project)
#      into the Firefox profile,
#   3. writes the two static Firefox prefs userChrome.css and the autoconfig
#      sandbox need (user.js; user prefs load before autoconfig).
#
# fx-autoconfig pin: dfdab5684faffc112b76ccb1d8cab7f75da0102c (v0.10.16).
# To update the loader, replace ./config.js and ./chrome/utils/* with the
# matching files from the pinned fx-autoconfig tree.
#
# Note: the design's own settings (uc.islands.*) are NOT set here — they are
# managed by the in-browser settings UI and persist in the profile's
# prefs.js. Re-applying them on every start would wipe live changes.

{ pkgs, ... }:

{
    home.packages = [
        (pkgs.firefox.override {
            extraPrefsFiles = [ ./config.js ];
        })
    ];

    home.file = {
        ".config/mozilla/firefox/uyl3o7i2.default/chrome".source = ./chrome;
        ".config/mozilla/firefox/uyl3o7i2.default/user.js".text = ''
            user_pref("toolkit.legacyUserProfileCustomizations.stylesheets", true);
            user_pref("general.config.sandbox_enabled", false);
        '';
    };
}