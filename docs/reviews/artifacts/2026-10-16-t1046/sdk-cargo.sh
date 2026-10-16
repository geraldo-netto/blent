#!/bin/sh
exec /usr/bin/flatpak run --user --unshare=network --no-session-bus --no-a11y-bus --no-documents-portal \
  --filesystem=/backups/disk2/projects/blent --filesystem=/tmp/blent-t1020-evidence --filesystem=/tmp/blent-t1046-evidence \
  --filesystem=/home/netto/.cargo --filesystem=/home/netto/.rustup --filesystem=host-os:ro \
  --env=BINDGEN_EXTRA_CLANG_ARGS=-resource-dir=/run/host/usr/lib/llvm-18/lib/clang/18 \
  --env=LIBCLANG_PATH=/run/host/usr/lib/llvm-18/lib \
  --env=LD_LIBRARY_PATH=/run/host/usr/lib/llvm-18/lib:/tmp/blent-t1020-evidence/clang-libs \
  --env=PATH=/home/netto/.cargo/bin:/usr/bin:/bin --env=CARGO_NET_OFFLINE=true \
  --env=CARGO_TARGET_DIR="$CARGO_TARGET_DIR" \
  --env=CARGO_LLVM_COV_TARGET_DIR="$CARGO_LLVM_COV_TARGET_DIR" \
  --env=CARGO_LLVM_COV_BUILD_DIR="$CARGO_LLVM_COV_BUILD_DIR" \
  --env=CARGO_BUILD_BUILD_DIR="$CARGO_BUILD_BUILD_DIR" \
  --command=/home/netto/.cargo/bin/cargo org.freedesktop.Sdk//25.08 "$@"
