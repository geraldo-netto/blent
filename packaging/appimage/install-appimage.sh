#!/usr/bin/env bash
# Explicit user registration; never remove a system package or activate EVDI.
set -euo pipefail
[[ $# -eq 0 ]] || { echo 'Usage: Blent.AppImage --install-user' >&2; exit 2; }
umask 077
[[ ${HOME:-} = /* ]] || { echo "HOME must be absolute." >&2; exit 2; }

absolute_base() {
    case "$1" in /*) printf '%s' "$1" ;; *) printf '%s' "$2" ;; esac
}
DATA_BASE=$(absolute_base "${XDG_DATA_HOME:-}" "$HOME/.local/share")
CONFIG_BASE=$(absolute_base "${XDG_CONFIG_HOME:-}" "$HOME/.config")
DEST="$DATA_BASE/blent/appimage"
TOOLS="$APPDIR/usr/share/blent"

# This subshell owns staging through renderer failure, interruption and rename.
# -T protects the exact destination even if its type changes after inspection.
publish_file() (
    local destination=$1 mode=$2 temporary
    shift 2
    temporary=$(mktemp "${destination%/*}/.blent.XXXXXXXX") || exit
    trap 'rm -f -- "$temporary"' EXIT
    "$@" > "$temporary" && chmod "$mode" "$temporary" &&
        [[ ! -d "$destination" ]] && mv -fT -- "$temporary" "$destination"
)

check_stopped() {
    local state
    state=$("$APPDIR/usr/bin/blent" status)
    case "$state" in
        *'blent is not running'*) ;;
        *) echo 'Stop the running Blent daemon before installing this AppImage.' >&2; exit 1 ;;
    esac
}

install_image() {
    publish_file "$DEST/Blent.AppImage" 755 cat "$APPIMAGE" || return
    LAUNCHER="$DEST/Blent.AppImage"
}

install_extracted() {
    local temporary previous="$DEST/Blent.AppDir.previous"
    [[ ! -e "$previous" ]] || { echo 'Resolve the previous interrupted AppDir installation first.' >&2; return 1; }
    temporary=$(mktemp -d "$DEST/.appdir.XXXXXXXX")
    if ! cp -a -- "$APPDIR/." "$temporary/"; then rm -rf -- "$temporary"; return 1; fi
    if [[ -e "$DEST/Blent.AppDir" ]]; then mv -- "$DEST/Blent.AppDir" "$previous"; fi
    if mv -- "$temporary" "$DEST/Blent.AppDir"; then
        rm -rf -- "$previous"
    else
        [[ ! -e "$previous" ]] || mv -- "$previous" "$DEST/Blent.AppDir"
        rm -rf -- "$temporary"
        return 1
    fi
    LAUNCHER="$DEST/Blent.AppDir/AppRun"
}

write_service() {
    local marker
    marker=$(printf '%s' "$LAUNCHER" | od -An -tx1 | tr -d ' \n') || return
    printf '# BLENT_APPIMAGE_PATH_HEX=%s\n' "$marker" || return
    bash "$TOOLS/write-systemd-service.sh" "$ENTRY" "$TOOLS/blent.service"
}

install_service() {
    mkdir -p -- "$CONFIG_BASE/systemd/user" || return
    publish_file "$CONFIG_BASE/systemd/user/blent.service" 600 write_service || return
    systemctl --user daemon-reload 2>/dev/null || true
}

install_desktop() {
    mkdir -p -- "$DATA_BASE/applications" "$DATA_BASE/icons/hicolor/scalable/apps" "$HOME/.local/bin" || return
    publish_file "$DATA_BASE/applications/blent.desktop" 600 bash "$TOOLS/write-desktop-entry.sh" "$ENTRY" "$TOOLS/blent.desktop" --gui || return
    cp -- "$APPDIR/usr/share/icons/hicolor/scalable/apps/blent.svg" "$DATA_BASE/icons/hicolor/scalable/apps/" || return
    ln -sfnT -- "$ENTRY" "$HOME/.local/bin/blent" || return
    install_gui_entry
}

backup_gui_entry() {
    local entry="$1" backup
    [[ -e "$entry" || -L "$entry" ]] || return 0
    [[ -f "$entry" || -L "$entry" ]] || { echo 'GUI launcher destination is not a file or link.' >&2; return 1; }
    backup=$(mktemp -d "$DEST/gui-backup.XXXXXXXX")
    mv -T -- "$entry" "$backup/blent-gui"
    printf '%s' "$backup/blent-gui"
}

link_gui_entry() {
    local entry="$HOME/.local/bin/blent-gui" backup
    if [[ -L "$entry" ]] && [[ $(readlink -- "$entry") = "$DEST/blent-gui" ]]; then return 0; fi
    backup=$(backup_gui_entry "$entry") || return
    if ln -sT -- "$DEST/blent-gui" "$entry"; then return 0; fi
    if [[ -n "$backup" ]]; then mv -T -- "$backup" "$entry"; fi
    return 1
}

write_gui_entry() {
    cat <<'GUI_SCRIPT'
#!/bin/sh
# BLENT_APPIMAGE_GUI_WRAPPER=1
set -eu
launcher=$(readlink -f -- "$0")
exec "${launcher%/*}/gui-launcher" --gui "$@"
GUI_SCRIPT
}

install_gui_entry() {
    # A sibling link avoids embedding user paths in executable shell text.
    ln -sfnT -- "$ENTRY" "$DEST/gui-launcher" || return
    publish_file "$DEST/blent-gui" 755 write_gui_entry || return
    link_gui_entry
}

write_entry() {
    cat <<'ENTRY_SCRIPT'
#!/bin/sh
set -eu
launcher=$(readlink -f -- "$0")
export APPIMAGE_EXTRACT_AND_RUN=1
exec "${launcher%/*}/Blent.AppImage" "$@"
ENTRY_SCRIPT
}

install_entry() {
    ENTRY=$LAUNCHER
    [[ $LAUNCHER = "$DEST/Blent.AppImage" ]] || return 0
    publish_file "$DEST/blent" 755 write_entry || return
    ENTRY="$DEST/blent"
}

update_autostart() {
    local entry="$CONFIG_BASE/autostart/blent.desktop"
    if systemctl --user is-enabled blent.service >/dev/null 2>&1; then
        mkdir -p -- "$CONFIG_BASE/autostart" || return
        publish_file "$entry" 600 cat "$TOOLS/blent-service-autostart.desktop"
    else
        [[ -f "$entry" ]] || return 0
        publish_file "$entry" 600 bash "$TOOLS/write-desktop-entry.sh" "$ENTRY" "$entry" start
    fi
}

check_destination() {
    local destination
    destination=$(realpath -m -- "$DEST")
    case "$destination/" in
        "$APPDIR/"*) echo 'Installation destination must be outside the source AppDir.' >&2; return 1 ;;
    esac
}

check_stopped
check_destination
mkdir -p -- "$DEST"
if [[ -n ${APPIMAGE:-} && -f $APPIMAGE ]]; then install_image; else install_extracted; fi
install_entry
install_service
install_desktop
update_autostart
printf 'Installed at %s\nAutostart preference is unchanged; enable it in Blent settings.\n' "$LAUNCHER"
