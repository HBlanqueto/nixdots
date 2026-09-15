These files contain the foundation of my daily driver setup, tailored for both personal use and software development. Here you will find what you are probably looking for: my desktop configuration.

This NixOS system features the following core components:

* **Desktop Environment:** [GNOME](https://www.gnome.org/)
* **Window Manager / Compositor:** [Hyprland](https://hyprland.org/)
* **Shell:** [Fish](https://fishshell.com/)
* **Terminal:** [Wezterm](https://wezfurlong.org/wezterm/)
* **Editor:** [Neovim](https://neovim.io/)

## Environment

- [x] **Flakes:** Fully flake-based system configuration
- [x] **Home Manager:** Declarative user environment
- [x] **Impermanence:** Ephemeral root using Btrfs subvolumes
- [x] **Lanzaboote & LUKS:** Secure Boot with Btrfs decryption
- [ ] **Agenix:** Secure secrets management

### Structure

An overview of the core configuration files and directories that make up this NixOS system:

```text
.
├── boot/                    # /boot — kernel, bootloader, impermanence rollback
│   ├── default.nix
│   └── snapshot.sh
├── etc/                     # /etc — NixOS system configuration
│   ├── desktop/
│   │   └── gnome.nix
│   ├── ai.nix               # ollama server + opencode AI stack
│   ├── default.nix
│   ├── fonts.nix
│   ├── global.nix
│   ├── services.nix
│   └── uutils.nix
├── home/                    # /home — home-manager configuration
│   └── humbe/               # one directory per user (matches username in settings.nix)
│       ├── config/
│       │   ├── fastfetch/
│       │   │   ├── config.jsonc
│       │   │   └── logo.txt
│       │   ├── mopidy.nix
│       │   ├── ncmpcpp.nix
│       │   ├── starship.nix
│       │   ├── wezterm.lua
│       │   └── yazi/
│       │       ├── default.nix
│       │       ├── icons.nix
│       │       ├── init.lua
│       │       └── yatline-icon.patch
│       ├── apps.nix
│       ├── default.nix
│       └── style.nix
├── share/                   # /usr/share — static assets
│   ├── themes/
│   │   ├── default.nix
│   │   ├── filecolors.nix
│   │   ├── neptunia.nix
│   │   └── vscode.nix
│   └── chime.wav
├── overlays/
│   └── default.nix
├── packages/
│   └── sf-mono-liga-bin.nix
├── flake.nix
├── flake.lock
├── hardware-configuration.nix
└── settings.nix
```

* **`flake.nix`:** The minimal main entry point for the system setup.
* **`settings.nix`:** Single source of truth for global user values (username, hostname, language, Git email).
* **`boot/`:** Startup settings, kernel, and Impermanence snapshot script.
* **`etc/`:** Core system environment (desktop, services, fonts, system-wide tools, and the local AI stack).
* **`home/`:** User-specific configurations (XDG) for each user in `settings.nix`, including app configs, programs (`apps.nix`), and theming (`style.nix`).
* **`share/`:** Static assets: the color themes and the boot audio.

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
git clone https://github.com/HBlanqueto/dotfiles.git
cd dotfiles

# Generate hardware config and copy it to the repository
nixos-generate-config --root /mnt
cp /mnt/etc/nixos/hardware-configuration.nix ~/dotfiles
```
### Impermanence

#### hardware-configuration.nix

Open `~/dotfiles/hardware-configuration.nix` and append `neededForBoot = true;` to the `/persist` and `/var/log` file systems:

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

For the boot rollback to work, `boot/snapshot.sh` must mount your exact Btrfs partition at the **top level** (`subvolid=5`), so that both `@rootfs` and `old_roots/` are visible.

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
> The script uses `awk` and `sort`, so make sure `gawk` is listed in `boot.initrd.systemd.initrdBin` in `boot/default.nix` (it is not provided by `uutils-coreutils`).

### Installation

> [!NOTE]
> If you changed the hostname in `settings.nix`, make sure to replace `#nixos` with your new hostname in the command below.

```shell
nixos-install --root /mnt --flake '#nixos' --impure --show-trace
```

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
