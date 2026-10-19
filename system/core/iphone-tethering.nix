{
  config,
  lib,
  pkgs,
  ...
}:

{
  # USB tethering from an iPhone (iOS "Personal Hotspot" over the cable).
  #
  # usbmuxd talks to the device and handles pairing/trust; once the phone is
  # trusted, the `ipheth` kernel module exposes the iPhone as a USB Ethernet
  # interface, which NetworkManager (already enabled) brings up and uses for
  # internet automatically. No manual `ip link`/DHCP needed.
  config = lib.mkIf config.keel.features.iphoneTethering {
    services.usbmuxd.enable = true;

    boot.kernelModules = [ "ipheth" ];

    # Handy for pairing/trusting the device and browsing its storage.
    environment.systemPackages = with pkgs; [
      libimobiledevice
      ifuse
    ];
  };
}
