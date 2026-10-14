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

An overview of the core configuration files and directories that make up this NixOS system:

```text
.
├── hosts/                   # one directory per host; auto-discovered by flake.nix
│   ├── thinkpad/            # physical L14 Gen 2a (x86_64-linux)
│   │   ├── host.nix         # per-host metadata: profile + system
│   │   ├── default.nix      # imports its hardware module + config
│   │   └── hardware-configuration.nix
│   └── utm/                 # UTM/QEMU VM on Apple Silicon (aarch64-linux)
│       ├── host.nix
│       ├── default.nix
│       └── hardware-configuration.nix
├── installer/               # Rust text-mode installer
├── lib/
│   └── mkHost.nix           # helper: mkHost { hostName, system, modules }
├── modules/                 # shared NixOS configuration
│   ├── common/              # users, fish, locale, nix, services, CLI tools
│   │   ├── ai.nix           # ollama server + opencode AI stack
│   │   ├── boot.nix         # generic boot/plymouth/sysctl
│   │   ├── default.nix
│   │   ├── fonts.nix
│   │   ├── home.nix         # home-manager wiring
│   │   ├── iphone-tethering.nix   # usbmuxd + ipheth (USB tethering)
│   │   ├── nix.nix          # nix/nixpkgs settings + impermanence
│   │   ├── services.nix
│   │   ├── user.nix
│   │   └── uutils.nix
│   ├── desktop/             # GNOME/GDM
│   │   ├── default.nix
│   │   └── gnome.nix
│   ├── hardware/            # host-specific hardware
│   │   ├── thinkpad.nix
│   │   ├── vm-utm.nix
│   │   └── snapshot.sh      # impermanence rollback (ThinkPad only)
│   └── home/                # home-manager configuration
│       ├── common/
│       │   ├── apps.nix
│       │   ├── default.nix
│       │   ├── style.nix
│       │   └── config/      # fastfetch, mopidy, starship, yazi
│       └── desktop/
│           ├── default.nix  # wezterm
│           └── wezterm.lua
├── share/                   # static assets: themes, boot chime
├── overlays/
│   └── default.nix
├── packages/
│   └── sf-mono-liga-bin.nix
├── flake.nix
├── flake.lock
├── disko.nix                # standalone disko layout used by the installer
├── test.sh                  # lints, Rust tests, flake build, TUI pty checks
└── settings.nix
```

* **`flake.nix`:** discovers every `hosts/<name>/host.nix` and emits one `nixosConfigurations.<name>` per host; also packages the installer for `x86_64-linux` and `aarch64-linux`.
* **`lib/mkHost.nix`:** builds a `nixosConfiguration` from `{ hostName, system, modules }`.
* **`settings.nix`:** host-independent values (username, locale, `hashedPasswordFile`, Git identity).
* **`hosts/<host>/host.nix`:** per-host metadata read by the flake (`profile` and `system`).
* **`hosts/<host>/default.nix`:** imports the host's `hardware-configuration.nix` and its hardware module.
* **`disko.nix`:** standalone disk layout (1 GiB ESP + btrfs subvolumes `@rootfs @nix @home @persist @log` plus `@rootfs-blank`) used by the installer.
* **`installer/`:** text-mode installer. See [installer/README.md](installer/README.md).
* **`test.sh`:** runs `cargo clippy`/`cargo test`, builds the flake outputs and drives both TUIs through a pty in simulation.
* **`modules/common` + `modules/desktop`:** shared system configuration.
* **`modules/hardware`:** ThinkPad/laptop and UTM/QEMU specifics.
* **`modules/home`:** shared home-manager configuration.

See [HOSTS.md](HOSTS.md) for the multi-host workflow and how to install the VM.

## Installer

`installer/` contains a text-mode installer (`nixdots-installer`). It is a Rust
binary that *orchestrates* existing tools — `disko`, `mkpasswd`,
`nixos-generate-config`, `sbctl`, `nix`, `nixos-install`, `nmcli`, ... — and
never reimplements partitioning or hashing.

```shell
nix run .#installer                 # simulation: no disk, no root
nix run .#installer -- --install    # real install from the NixOS ISO
```

The flake exposes it as `packages.{x86_64-linux,aarch64-linux}.{installer,default}`
and `apps.<system>.{installer,default}`. See
[installer/README.md](installer/README.md) for the full flow and options, and
`disko.nix` for the disk layout it applies.

## iPhone USB tethering

`modules/common/iphone-tethering.nix` enables iPhone internet sharing over the
USB cable: `services.usbmuxd` handles pairing/trust and the `ipheth` kernel
module exposes the phone as a USB Ethernet interface that NetworkManager brings
up automatically. `libimobiledevice` and `ifuse` are installed for pairing and
browsing the device. Once the iPhone is trusted, plug it in and enable
*Personal Hotspot* — no manual `ip link`/DHCP is required.

## Setup

> [!NOTE]
> I am not a Nix expert. 

> [!WARNING]
> This system may be considered unstable due to replacing `coreutils` with `uutils`. Disable this feature if necessary.

### Secure Boot (Lanzaboote)

> [!IMPORTANT]
> This configuration features a Zero-Touch Secure Boot setup.

Before booting the NixOS live USB, prepare your motherboard.

1. Enter your BIOS/UEFI settings.
2. Navigate to **Secure Boot**.
3. Select **Reset to Setup Mode** (or *Clear All Secure Boot Keys*).
4. Save and exit.

On the first boot after installation, Lanzaboote automatically generates the keys, bypasses `efivarfs` immutability to enroll them, and reboots the system. Because `/var/lib/auto-cryptenroll` and `/var/lib/sbctl` are persisted, no manual `sbctl` or `chattr` commands are ever required.

### Partitions

The quickest path is [`disko.nix`](disko.nix), which creates exactly the layout
below and is what the installer uses:

```shell
nix --experimental-features 'nix-command flakes' run github:nix-community/disko -- \
  --mode destroy,format,mount disko.nix --argstr device /dev/nvme0n1
```

To partition by hand instead:

```shell
# Mount the Btrfs root to create subvolumes
mount /dev/[your_partition] /mnt

# Create subvolumes
btrfs su cr /mnt/@{rootfs,home,nix,persist,log}

# Create a blank snapshot for Impermanence
btrfs subvolume snapshot -r /mnt/@rootfs /mnt/@rootfs-blank
umount /mnt

# Mount the ephemeral root
mount -o subvol=@rootfs,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt

# Create mount points
mkdir -p /mnt/{home,nix,persist,var/log,boot}

# Mount remaining subvolumes
mount -o subvol=@home,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/home
mount -o subvol=@nix,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/nix
mount -o subvol=@persist,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/persist
mount -o subvol=@log,noatime,compress=zstd,space_cache=v2 /dev/[your_partition] /mnt/var/log
```

### Generate hardware-configuration.nix

```shell
# Clone the repository
git clone https://github.com/HBlanqueto/nixdots.git
cd nixdots

# Generate the hardware config and copy it into the host directory
nixos-generate-config --root /mnt
cp /mnt/etc/nixos/hardware-configuration.nix hosts/thinkpad/   # or hosts/utm/
```
### Impermanence

#### hardware-configuration.nix

Open `hosts/<host>/hardware-configuration.nix` and append `neededForBoot = true;` to the `/persist` and `/var/log` file systems:

```nix
  fileSystems."/persist" = { 
    # ... your device and fsType ...
    options = [ "subvol=@persist" "noatime" "compress=zstd" "space_cache=v2" ];
    neededForBoot = true;
  };

  fileSystems."/var/log" = { 
    # ... your device and fsType ...
    options = [ "subvol=@log" "noatime" "compress=zstd" "space_cache=v2" ];
    neededForBoot = true;
  };
  ```

#### snapshot.sh

For the boot rollback to work, `modules/hardware/snapshot.sh` must mount your exact Btrfs partition at the **top level** (`subvolid=5`), so that both `@rootfs` and `old_roots/` are visible.

Replace the placeholder UUID with your drive's UUID (found in your generated `hardware-configuration.nix`):

```bash
#!/bin/sh

sleep 2
mkdir -p /btrfs_tmp

# Replace the UUID below with your actual partition UUID!
# subvolid=5 mounts the top level instead of the default subvolume.
mount -t btrfs -o subvolid=5 /dev/disk/by-uuid/YOUR-UUID-HERE /btrfs_tmp
```

Old snapshots cannot be deleted directly: systemd creates nested subvolumes inside the root (`srv`, `var/tmp`, `var/lib/portables`, `var/lib/machines`, ...) and Btrfs refuses to delete a parent subvolume while children exist. The script therefore prunes each expired snapshot **bottom-up**, deepest subvolume first:

```bash
# btrfs rejects deleting a subvolume that still contains children,
# so remove every nested subvolume first (deepest last).
btrfs subvolume list -o "$snapshot" | awk '{print $NF}' | sort -r \
  | while read -r path; do
      btrfs subvolume delete "/btrfs_tmp/$path"
    done

btrfs subvolume delete "$snapshot"
```

> [!NOTE]
> The script uses `awk` and `sort`, so make sure `gawk` is listed in `boot.initrd.systemd.initrdBin` in `modules/hardware/thinkpad.nix` (it is not provided by `uutils-coreutils`).

### Installation

The recommended path is the [installer](#installer): `nix run .#installer -- --install`
partitions with `disko.nix`, generates the hardware config and runs
`nixos-install` for you.

To install manually, the flake provides two hosts: `thinkpad` (x86_64-linux)
and `utm` (aarch64-linux). Replace `#thinkpad` with `#utm` when installing the
VM (see [HOSTS.md](HOSTS.md)).

```shell
nixos-install --root /mnt --flake ~/nixdots#thinkpad --impure --show-trace
```

> [!NOTE]
> `settings.nix` points `hashedPasswordFile` at `/persist/secrets/humbe.hash`.
> Create that file from the live ISO (e.g.
> `mkpasswd -m sha-512 > /mnt/persist/secrets/humbe.hash`) before the first rebuild.

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
