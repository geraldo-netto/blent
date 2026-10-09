#!/bin/sh
[ "$$" -eq 1 ] || exit 64
export PATH=/usr/bin:/bin:/usr/sbin
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
mkdir -p /dev/pts /run/udev /tmp/.X11-unix
mount -t devpts devpts /dev/pts
/usr/lib/systemd/systemd-udevd --daemon
/bin/modprobe virtio_pci
/bin/modprobe virtio_gpu
# Precreate before Xorg, as boot configuration does.
/bin/modprobe evdi initial_device_count=2
sleep 1
/usr/lib/xorg/Xorg :0 -noreset -nolisten tcp -ac -novtswitch -logfile /var/log/Xorg.log &
sleep 4
DISPLAY=:0 timeout 4 xdpyinfo >/tmp/ready 2>&1
printf 'T751 before query=%s count=' "$?"; cat /sys/devices/evdi/count
sh /setup-evdi.sh 2
printf 'T751 sufficient status=%s count=' "$?"; cat /sys/devices/evdi/count
sh /setup-evdi.sh 3
printf 'T751 growth status=%s count=' "$?"; cat /sys/devices/evdi/count
/helper-guard
printf 'T751 helper status=%s count=' "$?"; cat /sys/devices/evdi/count
DISPLAY=:0 timeout 4 xdpyinfo >/tmp/after 2>&1
printf 'T751 after query=%s\n' "$?"
cat /var/log/Xorg.log
poweroff -f
