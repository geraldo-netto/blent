# T1217: current package outputs and dependency setup

The package program builds AppImage, its corresponding source archive and the
current Arch recipe archive. Release upload/checksum and version-metadata callers
require that current set. Supported automatic dependency routes are Debian/Ubuntu
and Arch-family systems. Other IDs receive explicit manual dependency guidance
without changing repositories or installing packages. CI smoke targets match the
remaining current formats. No old spec, tool dependency, upload asset or native
lib64 package fallback is retained.

The permanent supported-output regression fails before repair on a required
obsolete input. Nine rejected-distro controls also fail before repair on unwanted
package operations. The exact tests then pass:7 package,14 installer,17 release,
1 complete notice-inventory,7 setup-hook and15 GPU-ordering tests. The notice,
release-version, stale-output, quoting/injection, current hook-order and package
inventory regressions remain permanent and keep their supported-format oracles.
Fake tools stay inside isolated fixtures; no real remote publication occurs.

The only changed maintained production function is install_distro_deps; native
Bash tracing measures4/4 executable lines (100%) with original source and extracted
function-body attestation. Other production edits delete unused functions or
change module-level packaging/CI wiring. The native source manifest, traces,
individual counter and exact source hashes are retained. Changed/new Python
functions score<=7 in the unchanged local rule model; maintained dispatch scores4
with the approved Shell standard. Native Sonar remains unavailable; this is not
native Sonar certification. All examined source scores remain<=9.

Dispatch borrows its original foreground shell argument and retains no state or
native handle. Supported package commands retain their existing caller ownership;
unknown families create none. One original package build funds its output tree,
marker and original AppImage/source processes; failure propagates before accepting
outputs. Its only remaining temporary Arch staging directory is held through the
synchronous tar operation and removed by the existing EXIT sponsor. One release
asset list owns current checksums/upload validation; its fake-API regression still
rejects every current upload failure. No new daemon, callback, background cleanup,
allocation protocol or compatibility path. No installation, live device/service,
Windows/native capture backend change, container download or network transfer.
Documentation/reference and metric retirement remain separate T1219/T1218 work.
