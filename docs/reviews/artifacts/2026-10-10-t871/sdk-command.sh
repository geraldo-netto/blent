set -eu
export PATH="/home/netto/.cargo/bin:$PATH"
export LIBCLANG_PATH=/tmp/blent-sdk-clang
export LD_LIBRARY_PATH=/tmp/blent-sdk-clang
export BINDGEN_EXTRA_CLANG_ARGS="-resource-dir /tmp/blent-sdk-clang/clang/18"
export CARGO_NET_OFFLINE=true CARGO_BUILD_JOBS=4
export CARGO_LLVM_COV_TARGET_DIR=/tmp/blent-t820-sdk-final-target
cd /backups/disk2/projects/blent
cargo llvm-cov show-env --sh > /tmp/blent-t871-sdk-env.sh
. /tmp/blent-t871-sdk-env.sh
mkdir -p /tmp/blent-t871-sdk-profiles
export LLVM_PROFILE_FILE='/tmp/blent-t871-sdk-profiles/%m-%p.profraw'
cargo test --offline --locked -p blent --features inproc-encoder --bin blent -- raw_socket::
cargo test --offline --locked -p blent --features inproc-encoder --bin blent -- raw_tests::t418_malformed_descriptors_stale_slots_and_peer_crash_are_bounded
