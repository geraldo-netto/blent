//! Stateful native ADB fixture for confirmed inventory and exact connect replies.
use std::{os::unix::fs::PermissionsExt, path::Path};
pub fn install(path: &Path, accept: bool) {
    let source = r#"#!/bin/sh
printf '%s\n' "$*" >> "$0.log"
case "$1" in
  devices)
    printf 'List of devices attached\n'
    if [ -s "$0.route" ]; then read -r route < "$0.route"; printf '%s\tdevice\n' "$route"; fi;;
  connect)
    printf '%s\n' "$2" > "$0.route"
    @REPLY@;;
  disconnect) : > "$0.route";;
  *) exit 85;;
esac
"#;
    let reply = if accept {
        r#"printf 'connected to %s\n' "$2""#
    } else {
        "printf 'disconnected\n'"
    };
    std::fs::write(path, source.replace("@REPLY@", reply)).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
}
