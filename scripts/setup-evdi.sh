#!/bin/sh
# Run as root. Loading is idempotent; never unload a live DRM device.
# Existing devices may belong to another display client and must survive upgrades.
evdi_setup_deferred() {
    echo "blent: $1; reboot after checking the evdi module and boot configuration, then run blent doctor" >&2
    return 1
}

evdi_device_count() {
    blent_count=$(cat /sys/devices/evdi/count 2>/dev/null) || return 1
    case "$blent_count" in ''|*[!0-9]*) return 1 ;; esac
    printf '%s\n' "$blent_count"
}

evdi_add_capacity() {
    printf '%s\n' "$1" | tee /sys/devices/evdi/add >/dev/null 2>&1 \
        || evdi_setup_deferred "could not add the missing EVDI devices"
}

evdi_safe_creation() {
    # A system-wide process check also covers setup launched outside the X session.
    # Missing/failed process inventory is not evidence that DRM hot-add is safe.
    pgrep -x 'Xorg|Xorg.wrap|X' >/dev/null 2>&1
    case "$?" in
        1) return 0 ;;
        0) evdi_setup_deferred "Xorg is running; configure initial_device_count for the next boot" ;;
        *) evdi_setup_deferred "cannot establish whether Xorg is running" ;;
    esac
}

evdi_capacity_ready() {
    blent_existing=$(evdi_device_count) || return 1
    [ "$blent_existing" -ge "$1" ] || return 1
    echo "blent: EVDI devices ready (count=$blent_existing); existing devices preserved"
}

evdi_complete_capacity() {
    blent_wanted=$1
    blent_existing=$(evdi_device_count) || {
        evdi_setup_deferred "cannot read the EVDI device count"
        return 1
    }
    if [ "$blent_existing" -lt "$blent_wanted" ]; then
        evdi_add_capacity "$((blent_wanted - blent_existing))" || return 1
        blent_existing=$(evdi_device_count) || {
            evdi_setup_deferred "cannot confirm the new EVDI capacity"
            return 1
        }
    fi
    if [ "$blent_existing" -lt "$blent_wanted" ]; then
        evdi_setup_deferred "required EVDI capacity is not available"
        return 1
    fi
    echo "blent: EVDI devices ready (count=$blent_existing); existing devices preserved"
}

evdi_setup() {
    if evdi_capacity_ready "$1"; then return 0; fi
    evdi_safe_creation || return 1
    modprobe evdi 2>/dev/null || {
        evdi_setup_deferred "evdi module is unavailable"
        return 1
    }
    evdi_complete_capacity "$1"
}

case "${1:-2}" in
    1|2|3|4) evdi_setup "${1:-2}" ;;
    *) echo "Usage: setup-evdi.sh [1-4]" >&2; exit 2 ;;
esac
