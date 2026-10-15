These files contain the foundation of my daily driver setup, tailored for both personal use and software development. The same flake builds **one configuration per host**: a physical **ThinkPad L14 Gen 2a** (`ThinkPad-L14-Gen2`, x86_64-linux) and a native **aarch64 UTM/QEMU VM** on Apple Silicon (`UTM-aarch64`).

This NixOS system features the following core components:

* **Desktop Environment:** [GNOME](https://www.gnome.org/) / GDM (Wayland)
* **Shell:** [Fish](https://fishshell.com/)
* **Terminal:** [Wezterm](https://wezfurlong.org/wezterm/)
* **Editor:** [Neovim](https://neovim.io/)

## Environment

- [x] **Flakes:** Fully flake-based system configuration
- [x] **Multi-host:** `ThinkPad-L14-Gen2` (x86_64-linux) and `UTM-aarch64` (aarch64-linux), auto-discovered from `hosts/`
- [x] **Installer:** text-mode installer (`installer/`, Rust)
- [x] **Home Manager:** Declarative user environment
- [x] **Impermanence:** Ephemeral root using Btrfs subvolumes
- [x] **Lanzaboote & LUKS:** Secure Boot with a LUKS2 root, auto-unlocked by Measured Boot (`systemd-pcrlock`) on the ThinkPad
- [x] **iPhone tethering:** USB internet sharing (`usbmuxd` + `ipheth`)
- [ ] **Agenix:** Secure secrets management

### Structure

```text
.
├── hosts/                   # one directory per host; auto-discovered by flake.nix
│   ├── ThinkPad-L14-Gen2/   # physical L14 Gen 2a (x86_64-linux)
│   └── UTM-aarch64/         # UTM/QEMU VM on Apple Silicon (aarch64-linux)
├── installer/               # Rust text-mode installer (see installer/README.md)
├── lib/mkHost.nix           # helper: mkHost { hostName, system, modules }
├── modules/                 # shared NixOS + home-manager configuration
│   ├── common/              # users, fish, locale, nix, services, iPhone, boot
│   ├── desktop/             # GNOME/GDM
│   ├── hardware/            # per-host hardware (thinkpad, utm)
│   └── home/                # home-manager (apps, style, wezterm, yazi…)
├── overlays/  packages/  share/
├── disko.nix                # standalone disk layout applied by the installer
├── settings.nix             # host-independent values (user, locale, git)
└── flake.nix  flake.lock
```

* **`flake.nix`:** discovers every `hosts/<name>/host.nix` and emits one `nixosConfigurations.<name>` per host; also packages the installer for `x86_64-linux` and `aarch64-linux`.
* **`settings.nix`:** host-independent values (username, locale, `hashedPasswordFile`, Git identity).
* **`hosts/<host>/`:** `host.nix` holds per-host metadata (`profile`, `system`); `default.nix` imports the host's hardware module and `hardware-configuration.nix`.
* **`disko.nix`:** disk layout applied by the installer (1 GiB ESP + a LUKS2 container `core` holding the btrfs subvolumes `@rootfs @nix @home @persist @log` plus `@rootfs-blank`).
* **`installer/`:** text-mode installer. See [installer/README.md](installer/README.md).
* **`test.sh`:** runs `cargo clippy`/`cargo test`, builds the flake outputs and drives both TUIs through a pty in simulation.

## Installer

`installer/` contains a text-mode installer (`nixdots-installer`). It is a Rust binary that *orchestrates* existing tools — `disko`, `mkpasswd`, `nixos-generate-config`, `sbctl`, `nix`, `nixos-install`, `nmcli`, ... — and never reimplements partitioning or hashing.

There are two ways to run it:

* **Normal (precompiled, no compilation):** download and run the release binary for your architecture.

  ```shell
  git clone --depth 1 https://github.com/HBlanqueto/nixdots ~/nixdots && sudo ~/nixdots/install.sh
  ```

  `install.sh` picks `x86_64`/`aarch64` from `uname -m`, downloads the binary from the [latest release](https://github.com/HBlanqueto/nixdots/releases/latest), verifies it against `SHA256SUMS` and runs it with `--install`. On a real VT it opens `kmscon` (Nerd Font + the neptunia palette) first. The binaries are built and published by `.github/workflows/installer-release.yml` on every `v*` tag.

* **Development (compiles from source):**

  ```shell
  sudo nix run .#installer -- --install   # real install from the NixOS ISO
  nix run .#installer                     # simulation: no disk, no root
  ```

The flake exposes it as `packages.{x86_64-linux,aarch64-linux}.{installer,default}` and `apps.<system>.{installer,default}`. See [installer/README.md](installer/README.md) for the full flow and options, and `disko.nix` for the disk layout it applies.

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

The recommended path is the [installer](#installer):
`git clone --depth 1 https://github.com/HBlanqueto/nixdots ~/nixdots && sudo ~/nixdots/install.sh`
(normal) or `sudo nix run .#installer -- --install` (development). It partitions
with `disko.nix` (no manual partitioning needed), generates the hardware config
and runs `nixos-install` for you.

To install manually, the flake provides two hosts: `ThinkPad-L14-Gen2` (x86_64-linux)
and `UTM-aarch64` (aarch64-linux). Replace `#ThinkPad-L14-Gen2` with `#UTM-aarch64`
when installing the VM.

```shell
nixos-install --root /mnt --flake ~/nixdots#ThinkPad-L14-Gen2 --impure --show-trace
```

### Disk encryption (LUKS + TPM)

`disko.nix` formats the root as a **LUKS2** container (`core`) holding the btrfs
subvolumes; only the ESP stays unencrypted (Lanzaboote must read the kernel and
initrd before unlocking). The installer uses the account password as the LUKS
passphrase, so early boots ask for it.

On the **ThinkPad**, once reinstalled with encryption, `modules/hardware/luks-tpm.nix`
enables Lanzaboote's **Measured Boot** (`systemd-pcrlock`): the passphrase is
enrolled into the TPM2 chip, bound to the firmware, bootloader and Secure Boot
PCRs (`0 4 7`), so boots unlock without typing. The `pcrlock` policy and
directory are persisted (`/var/lib/systemd/pcrlock.json`, `/var/lib/pcrlock.d`)
because the root is ephemeral. The UTM VM has no TPM and keeps asking for the
passphrase.

> [!NOTE]
> Verify support with
> `/run/current-system/systemd/lib/systemd/systemd-pcrlock is-supported`
> (must print `yes`). If it does not, leave `nixdots.tpmUnlock.enable = false;`.

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
