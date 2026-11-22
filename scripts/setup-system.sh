#!/usr/bin/env bash
# One privileged setup transaction. Existing EVDI configuration/devices stay owned.
set -euo pipefail

setup_warning() {
    printf 'blent setup: %s\n' "$1" >&2
}

preserve_evdi_config() (
    local source=$1 destination=$2 temporary
    if [[ -e $destination || -L $destination ]]; then return 0; fi
    temporary=$(mktemp "${destination%/*}/.blent-evdi.XXXXXXXX") || return
    trap 'rm -f -- "$temporary"' EXIT
    install -m644 -- "$source" "$temporary" || return
    if ln -T -- "$temporary" "$destination" 2>/dev/null; then return 0; fi
    [[ -e $destination || -L $destination ]]
)

configure_boot_modules() {
    local config=$1 resources=$2 scripts=$3 drm=$4 init=$5
    mkdir -p -- "$config/modprobe.d" "$config/modules-load.d" || return
    preserve_evdi_config "$resources/blent-evdi.conf" "$config/modprobe.d/blent-evdi.conf" || return
    install -m644 -- "$resources/blent-modules.conf" "$config/modules-load.d/blent.conf" || return
    bash "$scripts/gpu-boot-order.sh" "$drm" "$config/modprobe.d" \
        || setup_warning 'GPU boot ordering needs manual review; see docs/installation.md.'
    if [[ ! -d $init ]]; then
        setup_warning 'Configure your init system to load evdi and uinput at boot; modules-load.d support is not guaranteed.'
    fi
}

configure_uinput() {
    local config=$1 rule=$2
    install -Dm644 -- "$rule" "$config/udev/rules.d/60-blent-uinput.rules" || return
    modprobe uinput 2>/dev/null || setup_warning 'uinput needs manual activation.'
    udevadm control --reload 2>/dev/null || setup_warning 'Device rules need manual activation: reload your device manager.'
    udevadm trigger --name-match=uinput 2>/dev/null || setup_warning 'uinput permissions need manual activation: trigger your device manager.'
}

activate_evdi() {
    bash "$1/setup-evdi.sh" 2 \
        || setup_warning 'EVDI activation deferred; existing devices were preserved. Reboot after reviewing module setup.'
}

setup_system() {
    local config=$1 resources=$2 scripts=$3 drm=$4 init=$5
    configure_boot_modules "$config" "$resources" "$scripts" "$drm" "$init" || return
    configure_uinput "$config" "$resources/60-blent-uinput.rules" || return
    activate_evdi "$scripts"
    printf 'blent setup: configuration installed; review any activation warnings above.\n'
}

system_setup_main() {
    if [[ $# != 1 ]]; then printf 'Usage: setup-system.sh RESOURCE_DIRECTORY\n' >&2; return 2; fi
    local scripts
    scripts=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd) || return
    setup_system /etc "$1" "$scripts" /sys/class/drm /run/systemd/system
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then system_setup_main "$@"; fi
