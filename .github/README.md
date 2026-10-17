These files contain the foundation of my daily driver setup, tailored for both personal use and software development. The flake builds **one generic configuration** (`nixdots`): the hardware is detected at evaluation time by [nixos-facter](https://github.com/nix-community/nixos-facter) and the per-machine choices (hostname, encryption, Secure Boot, peripherals) live in `machine.json` under `/persist`. The same output name is also exposed as the machine's hostname, so `nixos-rebuild --flake ~/nixdots` without `#…` works.

This NixOS system features the following core components:

* **Desktop Environment:** [GNOME](https://www.gnome.org/) / GDM (Wayland)
* **Shell:** [Fish](https://fishshell.com/)
* **Terminal:** [Wezterm](https://wezfurlong.org/wezterm/)
* **Editor:** [Neovim](https://neovim.io/)

## Environment

- [x] **Flakes:** Fully flake-based system configuration
- [x] **Single config:** one generic `nixdots` output; hardware detected by facter, per-machine facts in `machine.json`
- [x] **Installer:** text-mode installer (`installer/`, Rust)
- [x] **Home Manager:** Declarative user environment
- [x] **Impermanence:** Ephemeral root using Btrfs subvolumes
- [x] **Lanzaboote & LUKS:** Secure Boot with a LUKS2 root, auto-unlocked by Measured Boot (`systemd-pcrlock`) on the ThinkPad
- [x] **iPhone tethering:** USB internet sharing (`usbmuxd` + `ipheth`)
- [ ] **Agenix:** Secure secrets management

### Structure

```text
.
├── installer/               # Rust text-mode installer (see installer/README.md)
├── lib/mkHost.nix           # helper: mkHost { hostName, system, modules, data }
├── modules/                 # shared NixOS + home-manager configuration
│   ├── common/              # users, fish, locale, nix, services, hardware-detect, boot
│   ├── desktop/             # GNOME/GDM
│   ├── hardware/            # boot rollback snapshot helper
│   └── home/                # home-manager (apps, style, wezterm, yazi…)
├── overlays/  packages/  share/
├── disko.nix                # standalone disk layout applied by the installer
├── settings.nix             # machine-independent values (locale, keyboard, stateVersion)
└── flake.nix  flake.lock
```

* **`flake.nix`:** emits the generic `nixdots` configuration (plus pure CI variants and the first-boot test VMs); also packages the installer for `x86_64-linux` and `aarch64-linux`.
* **`settings.nix`:** machine-independent values (locale, keyboard, `stateVersion`).
* **`machine.json` / `user.json` / `facter.json`:** per-machine data under `/persist/nixdots/`; the installer writes `machine.json` and `facter.json`, the first-boot assistant writes `user.json` (plus the password hash), and `lib/data.nix` merges them.
* **`disko.nix`:** disk layout applied by the installer (1 GiB ESP + a LUKS2 container `core` holding the btrfs subvolumes `@rootfs @nix @home @persist @log` plus `@rootfs-blank`).
* **`installer/`:** text-mode installer. See [installer/README.md](installer/README.md).
* **`test.sh`:** runs `cargo clippy`/`cargo test`, builds the flake outputs and drives both TUIs through a pty in simulation.

## Installer

`installer/` contains a text-mode installer (`nixdots-installer`). It is a Rust binary that *orchestrates* existing tools — `disko`, `nixos-facter`, `sbctl`, `nix`, `nixos-install`, `nmcli`, ... — and never reimplements partitioning.

There are two ways to run it:

* **Normal (precompiled, no compilation):** clone the repo and run the bootstrap, which downloads and runs the release binary for your architecture.

  ```shell
  nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
  sudo ./nixdots/install.sh
  ```

  `install.sh` picks `x86_64`/`aarch64` from `uname -m`, downloads the binary from the [latest release](https://github.com/HBlanqueto/nixdots/releases/latest), verifies it against `SHA256SUMS` and runs it with `--install`, providing (via `nix shell`, pinned to `flake.lock`) the tools the live ISO lacks. On a real VT it opens `kmscon` (Nerd Font + the neptunia palette) first. The binaries are built and published by `.github/workflows/installer-release.yml` on every `v*` tag.

The installed system's TTY uses **only** the XNU (macOS text-console) font, read by kmscon's `psf` engine at double size, which has 256 Latin-1 glyphs; on the VT the fish prompt (starship) and `eza` therefore run without Nerd Font icons, and characters outside Latin-1 come out as «?». The installer's own VT keeps Liga SFMono Nerd Font 14.

* **Development (compiles from source):**

  ```shell
  sudo nix run .#installer -- --install   # real install from the NixOS ISO
  nix run .#installer                     # simulation: no disk, no root
  ```

The flake exposes it as `packages.{x86_64-linux,aarch64-linux}.{installer,default}` and `apps.<system>.{installer,default}`. See [installer/README.md](installer/README.md) for the full flow and options, and `disko.nix` for the disk layout it applies.

## Setup

> [!NOTE]
> I am not a Nix expert.

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

```shell
nix-shell -p git --run "git clone --depth 1 https://github.com/HBlanqueto/nixdots"
sudo ./nixdots/install.sh
```

(development: `sudo nix run .#installer -- --install`). It partitions with
`disko.nix` (no manual partitioning needed), runs `nixos-facter`, writes
`machine.json` and runs `nixos-install --flake <repo>#nixdots` for you. The
account is **not** created here: on the first reboot a Slint assistant asks for
the user, the password and git, writes `user.json` and the hash, and rebuilds.

To install manually, the flake provides the single `nixdots` configuration:

```shell
nixos-install --root /mnt --flake ~/nixdots#nixdots --impure --show-trace
```

### Disk encryption (LUKS + TPM)

`disko.nix` formats the root as a **LUKS2** container (`core`) holding the btrfs
subvolumes; only the ESP stays unencrypted (Lanzaboote must read the kernel and
initrd before unlocking). The installer asks for a dedicated LUKS passphrase on
the **Cifrado** screen (separate from the account password, which does not exist
yet) and early boots ask for it.

On the **ThinkPad**, once reinstalled with encryption, `modules/common/luks-tpm.nix`
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
> The password hash is written by the first-boot assistant to
> `/persist/secrets/<user>.hash` (`0600`, root-owned), creating the directory
> with `0711` so the unprivileged `nix` evaluation can still stat the file
> (`nixdots-rebuild` builds without sudo). It is never committed to the repo.

### Impermanence

Two details matter when partitioning by hand (the installer handles them):

- Append `neededForBoot = true;` to the `/persist` and `/var/log` file systems (NixOS already marks `/persist` and `/var/log` in `modules/common/filesystems.nix`).
- For boot rollback, `modules/hardware/snapshot.sh` must mount the Btrfs partition at the top level (`subvolid=5`, via the drive's UUID) so both `@rootfs` and `old_roots/` are visible; expired snapshots are pruned bottom-up (deepest nested subvolume first), because Btrfs refuses to delete a parent subvolume while children exist.

## Credits

Special thanks to these individuals whose work greatly inspired and guided the configuration of these files:

* [SergioRibera's dotfiles](https://github.com/SergioRibera/dotfiles)
* [chxp82q's nix-config](https://github.com/chxp82q/nix-config)
