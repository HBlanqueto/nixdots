<div align="center">

  # Nixdots
</div>


</div>

 <a><img alt="NixOS Logo" height="155" align = "right" src="https://raw.githubusercontent.com/NixOS/nixos-artwork/521e1b0a899074ca7a701c17e357c63c13d54133/logo/nix-snowflake.svg"></a>

 Welcome, as you can see I'm using NixOS as my daily operating system. This repo contains a collection of files configured for an `ASUS-C400SA`, most part of these settings have been thinking for using Gnome Desktop with Wayland session, my second option is AwesomeWM. Feel free to modify and use my dots however you decide.

 > I'm implementing [Flakes](https://nixos.wiki/wiki/Flakes#:~:text=Nix%20flakes%20is%20some%20upcoming%20feature%20in%20the,flake.nix%20where%20they%20can%20describe%20their%20own%20dependencies.) to make installation easy, do not use my files as a template. I haven't understood all things right.

<div>

## Setup for NixOS

1.  Get the latest [ISO](https://nixos.org/download.html#nixos-iso) and boot into the enviroment.
2.  Make your partition scheme according to your needs & format them with your desired filesystem.

> I shall use ZFS, if you're planning to use another filesystem, skip this step.


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

# Get into a Nix shell with Nix unstable and git
nix-shell -p git nixUnstable

# Clone my dotfiles
git clone https://github.com/HBlanqueto/nixdots /mnt/etc/nixos 

# Remove this file
rm /mnt/etc/nixos/hosts/asus-s400ca/hardware-configuration.nix

# Follow the next commands to make sure to generate
# the hardware-configuration file acording to your computer
# and delete configuration.nix to avoid problems
nixos-generate-config --root /mnt
cp /mnt/etc/nixos/hardware-configuration.nix /mnt/etc/nixos/hosts/asus-s400ca/
rm /mnt/etc/nixos/configuration.nix

# Make sure you're in the configuration directory
cd /mnt/etc/nixos

# Install this NixOS configuration with flakes
nixos-install --flake '.#laptop'

```
4. Configure the password and boot the system.
5. Once boot, configure home-manager files.

```sh
# change ownership of configuration folder
sudo chown -R $USER /etc/nixos

# go into the configuration folder
cd /etc/nixos

# Install the home manager configuration
home-manager switch --flake '.#myself'
```

## Gallery

### Gnome Desktop

I love using vanilla Gnome, two things to destacate is the GTK3 theme [adw-gtk3](https://github.com/lassekongo83/adw-gtk3) & the [Firefox userchrome](https://github.com/HBlanqueto/safadwaita) which I forked  

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-01.png" alt="img" align="center" width="800px">
</div>

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-02.png" alt="img" align="center" width="800px">
</div>

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/Gnome-03.png" alt="img" align="center" width="800px">
</div>

### Awesome Window Manager

**Modules**
- [Bling](https://blingcorp.github.io/bling/#/) for tabbing and the tiled wallpaper.

<div align=center>
<img src="https://raw.githubusercontent.com/HBlanqueto/nixdots/snowflake/.github/images/awesomewm.png" alt="img" align="center" width="800px">
</div>

## TODO
- [x] Add flakes files.
- [ ] Finish AwesomeWM rice.
- [ ] Upload this on unixporn.
- [ ] Improve nix configuration.
- [x] Be happy.

## Special thanks

- [dotfiles](https://github.com/JavaCafe01/dotfiles) by **javacafe01**
- [VitalConfig](https://github.com/Leznom/VitalConfig) by **Leznom**
- [nixdots](https://github.com/jx11r/nixdots) by **jx11r**

