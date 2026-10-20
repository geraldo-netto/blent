#!/bin/busybox sh
set -eu
/bin/busybox --install -s /bin
export PATH=/bin
trap 'echo T1185_INIT_EXIT; poweroff -f' EXIT
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev
insmod /modules/ttm.ko
insmod /modules/drm_ttm_helper.ko
insmod /modules/evdi.ko initial_device_count=1
echo T1185_DRIVER_READY
if /pause-flip-probe modeset; then result=0; else result=$?; fi
echo T1185_PROBE_EXIT=$result
cat /helper.log
echo 1 > /sys/devices/evdi/remove_all
rmmod evdi
echo T1185_MODULE_RETIRED
dmesg
echo T1185_GUEST_DONE
