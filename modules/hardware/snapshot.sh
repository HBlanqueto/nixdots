#!/bin/sh

    sleep 2
    
    mkdir -p /btrfs_tmp

    mount -t btrfs -o subvolid=5 /dev/disk/by-uuid/9ddcd09a-903e-4863-94db-bb1ccb5d3236 /btrfs_tmp

    if [ -e /btrfs_tmp/@rootfs ]; then
      mkdir -p /btrfs_tmp/old_roots
      timestamp=$(date "+%Y-%m-%d_%H-%M-%S")
      
      mv -f /btrfs_tmp/@rootfs /btrfs_tmp/old_roots/${timestamp}
    fi

    if [ -d /btrfs_tmp/old_roots ]; then
      for snapshot in /btrfs_tmp/old_roots/*/; do
        [ -d "$snapshot" ] || continue
        [ "$(find "$snapshot" -maxdepth 0 -mtime +30)" ] || continue

        # btrfs refuses to delete a subvolume containing immutable inodes
        # (e.g. /var/empty from openssh), so clear the attribute first.
        # This descends into nested subvolumes too, since they share the
        # same block device.
        chattr -R -i "$snapshot" 2>/dev/null || true

        # btrfs rejects deleting a subvolume that still contains children
        # (systemd creates srv, var/tmp, var/lib/portables, ...), so remove
        # every nested subvolume first (deepest last) and then the snapshot.
        btrfs subvolume list -o "$snapshot" | awk '{print $NF}' | sort -r \
          | while read -r path; do
              btrfs subvolume delete "/btrfs_tmp/$path" 2>/dev/null || true
            done

        btrfs subvolume delete "$snapshot" 2>/dev/null \
          || echo "rollback: failed to prune $snapshot" >&2
      done
    fi

    btrfs subvolume snapshot /btrfs_tmp/@rootfs-blank /btrfs_tmp/@rootfs

    umount /btrfs_tmp