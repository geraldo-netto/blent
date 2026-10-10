#!/bin/sh
# Run from repository root inside the installed offline SDK, with --device=dri.
set -eu
probe_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
result_dir=${1:?supply output directory}
probe_cpus=${T794_CPUSET:-8-11}
probe_frames=${T794_FRAMES:-200}
mkdir -p "$result_dir/bin" "$result_dir/pairs" "$result_dir/sharing"

build_probes() {
    for probe in mapped share isolated; do
        cc -O3 -ffunction-sections -fdata-sections \
            -Ihost/evdi -Ihost/gpu -Iscripts/benchmarks \
            $(pkg-config --cflags libavcodec libavutil libva libdrm x11 xrandr) \
            "$probe_dir/$probe.c" -o "$result_dir/bin/$probe" \
            -Wl,--gc-sections $(pkg-config --libs libavcodec libavutil libva) -lpthread
    done
}

measure_node() {
    node=$1
    for pair in 1 2 3 4 5; do
        if test $((pair % 2)) -eq 1; then modes="0 1"; else modes="1 0"; fi
        for mode in $modes; do
            base="$result_dir/pairs/n${node}-p${pair}-m${mode}"
            taskset -c "$probe_cpus" "$result_dir/bin/isolated" "/dev/dri/renderD$node" 1280 800 "$mode" "$probe_frames" "$base.h264" > "$base.json"
        done
    done
}

check_surface_size() {
    node=$1; width=$2; height=$3
    base="$result_dir/sharing/n${node}-${width}x${height}"
    for mode in 0 1; do
        "$result_dir/bin/share" "/dev/dri/renderD$node" "$width" "$height" "$mode" "$base-m$mode.h264" > "$base-m$mode.log" 2>&1
        ffmpeg -hide_banner -loglevel error -threads 1 -i "$base-m$mode.h264" -f framemd5 "$base-m$mode.framemd5"
    done
    cmp "$base-m0.h264" "$base-m1.h264"
    cmp "$base-m0.framemd5" "$base-m1.framemd5"
}

build_probes
for node in 128 129; do measure_node "$node"; done
for node in 128 129; do
    check_surface_size "$node" 1280 800
    check_surface_size "$node" 1282 802
    for mode in 0 1; do
        ffmpeg -hide_banner -loglevel error -threads 1 -i "$result_dir/pairs/n${node}-p1-m$mode.h264" -f framemd5 "$result_dir/pairs/n${node}-m$mode.framemd5"
    done
    cmp "$result_dir/pairs/n${node}-m0.framemd5" "$result_dir/pairs/n${node}-m1.framemd5"
done
python3 "$probe_dir/summarize.py" "$result_dir/pairs" > "$result_dir/summary.json"
