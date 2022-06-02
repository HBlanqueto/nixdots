<div align="center">

  # Nixdots
</div>


</div>

 <a><img alt="NixOS Logo" height="155" align = "right" src="https://raw.githubusercontent.com/NixOS/nixos-artwork/521e1b0a899074ca7a701c17e357c63c13d54133/logo/nix-snowflake.svg"></a>

 Welcome, as you'll see I'm currently using NixOS as main system. This files are specifically for an `ASUS-C400SA`, most part of these settings have been thinking for using Gnome Desktop with Wayland session, my second option is AwesomeWM. Feel free to modify and use my dots for your own use.

 > I want to implement [Flakes](https://nixos.wiki/wiki/Flakes#:~:text=Nix%20flakes%20is%20some%20upcoming%20feature%20in%20the,flake.nix%20where%20they%20can%20describe%20their%20own%20dependencies.) to make installation easy, do not use my files as a template. I haven't understood It at all.

<div>

## Setup for NixOS

1.  Get the latest [ISO](https://nixos.org/download.html#nixos-iso) and boot into the enviroment.
2.  Make your partitions acording and assing a file system.

> I use ZFS, If you are planning using another file system; jump this step.


<details>
<summary><b>ZFS Partitions</b></summary>
<br>

I like using ZFS as a personal challenge, I took as a reference [Dan's Cheat Sheets](https://cheat.readthedocs.io/en/latest/nixos/zfs_install.html), try this by your own risk.

```sh
# Create 2 partitions.
cfdisk /dev/sda

# Prepare ZFS on /dev/sdaX. "X" could be changed to another number.
zpool create -f -O mountpoint=none \ 
-O atime=off \ 
-O compression=zstd \
-O xattr=sa \ 
-O acltype=posixacl \
-o ashift=12 \
-R /mnt \
rpool /dev/sdaX

# Create /root & /home subvolumes.
zfs create -o mountpoint=none rpool/root
zfs create -o mountpoint=legacy rpool/root/nixos
zfs create -o mountpoint=legacy rpool/home

# Mount the /root and /home.
mount -t zfs rpool/root/nixos /mnt
mkdir /mnt/home
mount -t zfs rpool/home /mnt/home

# Create and mount /boot.
mkfs.vfat /dev/sdaX
mkdir /mnt/boot
mount /dev/sdaX /mnt/boot
```

</details>
‎‎

3. Follow the next commands to download and prepare the installation.

```sh

# Generate initial configuration.
nixos-generate-config --root /mnt

# Copy the hardware-configuration.nix file to ~/
# This is totally necesary for the next steps.
cp /mnt/etc/nixos/hardware-configuration.nix ~/

# # Get into a Nix shell with git.
nix-shell -p git

# Clone the repository & copy the files.
# Remember to change modules and drivers relative to hardaware's file you have in ~/
git clone https://github.com/HBlanqueto/nixdots.git --depth 1
cp -r ~/nixdots/nixos /mnt/etc/

# Once you modified the files. Install the OS.
nixos-install --root /mnt

```
4. Configure the password and boot the system.
5. Once boot, add unstable channel just because.

```sh

# Not using sudo only affects users (user packages).
nix-channel --add https://nixos.org/channels/nixos-unstable nixos

# Update the channel.
nix-channel --update

# Rebuild the system.
nixos-rebuild switch --upgrade
```

## Gallery

**Gnome Desktop**

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-01.png" alt="img" align="center" width="800px">
</div>

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-02.png" alt="img" align="center" width="800px">
</div>

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-03.png" alt="img" align="center" width="800px">
</div>

**Awesome Window Manager**

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/awesomewm.png" alt="img" align="center" width="800px">
</div>

## Special thanks

- [dotfiles](https://github.com/JavaCafe01/dotfiles) by **javacafe01**
- [VitalConfig](https://github.com/Leznom/VitalConfig) by **Leznom**
- [nixdots](https://github.com/jx11r/nixdots) by **jx11r**

