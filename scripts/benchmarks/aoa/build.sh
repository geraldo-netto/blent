#!/bin/sh
# Offline build only. Running the resulting binary changes the chosen USB mode.
set -eu
probe_output=${1:?usage: build.sh OUTPUT_DIRECTORY}
probe_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
probe_headers=${BLENT_LIBUSB_INCLUDE:-/home/netto/.local/share/flatpak/runtime/org.freedesktop.Sdk/x86_64/25.08/active/files/include/libusb-1.0}
test -f "$probe_headers/libusb.h"
mkdir -p "$probe_output"
cc -std=c11 -O2 -g -Wall -Wextra -Werror -I"$probe_headers" \
    "$probe_root/scripts/benchmarks/aoa/main.c" \
    "$probe_root/scripts/benchmarks/aoa/usb_probe.c" \
    "$probe_root/scripts/benchmarks/aoa/contract.c" \
    -l:libusb-1.0.so.0 -lz -o "$probe_output/aoa-probe"
