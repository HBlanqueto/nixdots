These files contain the foundation of my daily driver setup, tailored for both personal use and software development. The same flake builds **one configuration per host**: a physical **ThinkPad L14 Gen 2a** (`thinkpad`, x86_64-linux) and a native **aarch64 UTM/QEMU VM** on Apple Silicon (`utm`).

This NixOS system features the following core components:

* **Desktop Environment:** [GNOME](https://www.gnome.org/) / GDM (Wayland)
* **Shell:** [Fish](https://fishshell.com/)
* **Terminal:** [Wezterm](https://wezfurlong.org/wezterm/)
* **Editor:** [Neovim](https://neovim.io/)

## Environment

- [x] **Flakes:** Fully flake-based system configuration
- [x] **Multi-host:** `thinkpad` (x86_64-linux) and `utm` (aarch64-linux), auto-discovered from `hosts/`
- [x] **Installer:** text-mode installer (`installer/`, Rust)
- [x] **Home Manager:** Declarative user environment
- [x] **Impermanence:** Ephemeral root using Btrfs subvolumes
- [x] **Lanzaboote & LUKS:** Secure Boot with Btrfs decryption
- [x] **iPhone tethering:** USB internet sharing (`usbmuxd` + `ipheth`)
- [ ] **Agenix:** Secure secrets management

### Structure

```text
.
├── hosts/                   # one directory per host; auto-discovered by flake.nix
│   ├── thinkpad/            # physical L14 Gen 2a (x86_64-linux)
│   └── utm/                 # UTM/QEMU VM on Apple Silicon (aarch64-linux)
├── installer/               # Rust text-mode installer (see installer/README.md)
├── lib/mkHost.nix           # helper: mkHost { hostName, system, modules }
├── modules/                 # shared NixOS + home-manager configuration
│   ├── common/              # users, fish, locale, nix, services, iPhone, boot
│   ├── desktop/             # GNOME/GDM
│   ├── hardware/            # per-host hardware (thinkpad, vm-utm)
│   └── home/                # home-manager (apps, style, wezterm, yazi…)
├── overlays/  packages/  share/
├── disko.nix                # standalone disk layout applied by the installer
├── settings.nix             # host-independent values (user, locale, git)
└── flake.nix  flake.lock
```

* **`flake.nix`:** discovers every `hosts/<name>/host.nix` and emits one `nixosConfigurations.<name>` per host; also packages the installer for `x86_64-linux` and `aarch64-linux`.
* **`settings.nix`:** host-independent values (username, locale, `hashedPasswordFile`, Git identity).
* **`hosts/<host>/`:** `host.nix` holds per-host metadata (`profile`, `system`); `default.nix` imports the host's hardware module and `hardware-configuration.nix`.
* **`disko.nix`:** disk layout applied by the installer (1 GiB ESP + btrfs subvolumes `@rootfs @nix @home @persist @log` plus `@rootfs-blank`).
* **`installer/`:** text-mode installer. See [installer/README.md](installer/README.md).
* **`test.sh`:** runs `cargo clippy`/`cargo test`, builds the flake outputs and drives both TUIs through a pty in simulation.

## Installer

`installer/` contains a text-mode installer (`nixdots-installer`). It is a Rust binary that *orchestrates* existing tools — `disko`, `mkpasswd`, `nixos-generate-config`, `sbctl`, `nix`, `nixos-install`, `nmcli`, ... — and never reimplements partitioning or hashing.

```shell
nix run .#installer                 # simulation: no disk, no root
nix run .#installer -- --install    # real install from the NixOS ISO
```

The flake exposes it as `packages.{x86_64-linux,aarch64-linux}.{installer,default}` and `apps.<system>.{installer,default}`. See [installer/README.md](installer/README.md) for the full flow and options, and `disko.nix` for the disk layout it applies.

## iPhone USB tethering

`modules/common/iphone-tethering.nix` enables iPhone internet sharing over the USB cable: `services.usbmuxd` handles pairing/trust and the `ipheth` kernel module exposes the phone as a USB Ethernet interface that NetworkManager brings up automatically. `libimobiledevice` and `ifuse` are installed for pairing and browsing the device. Once the iPhone is trusted, plug it in and enable *Personal Hotspot* — no manual `ip link`/DHCP is required.

## Setup

> [!NOTE]
> I am not a Nix expert.

> [!WARNING]
> This system may be considered unstable due to replacing `coreutils` with `uutils`. Disable this feature if necessary.

### Secure Boot (Lanzaboote)

> [!IMPORTANT]
> This configuration features a Zero-Touch Secure Boot setup.

Before booting the NixOS live USB, prepare your motherboard:

1. Enter your BIOS/UEFI settings.
2. Navigate to **Secure Boot**.
3. Select **Reset to Setup Mode** (or *Clear All Secure Boot Keys*).
4. Save and exit.

On the first boot after installation, Lanzaboote automatically generates the keys, bypasses `efivarfs` immutability to enroll them, and reboots the system. Because `/var/lib/auto-cryptenroll` and `/var/lib/sbctl` are persisted, no manual `sbctl` or `chattr` commands are ever required.

### Installation

The recommended path is the [installer](#installer): `nix run .#installer -- --install`
partitions with `disko.nix` (no manual partitioning needed), generates the
hardware config and runs `nixos-install` for you.

To install manually, the flake provides two hosts: `thinkpad` (x86_64-linux)
and `utm` (aarch64-linux). Replace `#thinkpad` with `#utm` when installing the
VM.

```shell
nixos-install --root /mnt --flake ~/nixdots#thinkpad --impure --show-trace
```

> [!NOTE]
> `settings.nix` points `hashedPasswordFile` at `/persist/secrets/humbe.hash`.
> Create that file from the live ISO (e.g.
> `mkpasswd -m sha-512 > /mnt/persist/secrets/humbe.hash`) before the first rebuild.
> Keep the directory traversable (`chmod 0711 /persist/secrets`): the file is
> `0600` root-owned, but the unprivileged `nix` evaluation must still be able to
> stat it (`nixdots-rebuild` builds without sudo).

### Impermanence

Two details matter when partitioning by hand (the installer handles them):

- Append `neededForBoot = true;` to the `/persist` and `/var/log` file systems in `hosts/<host>/hardware-configuration.nix`.
- For boot rollback, `modules/hardware/snapshot.sh` must mount the Btrfs partition at the top level (`subvolid=5`, via the drive's UUID) so both `@rootfs` and `old_roots/` are visible; expired snapshots are pruned bottom-up (deepest nested subvolume first), because Btrfs refuses to delete a parent subvolume while children exist.

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
