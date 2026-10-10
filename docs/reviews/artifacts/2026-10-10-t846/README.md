# T846 offline prototype evidence

The original offline snapshot involved no USB mode switch, application installation,
debugging-setting change or host service restart. Production still requires ADB;
T846 remains open. Later native attempts are retained under `live/` and described in the owning
review. Attempt 5 passes the synthetic workload after the zero-length packet
fix; production and full lifecycle acceptance remain open.

- `enumeration-red.log.gz`: permanent regressions fail against the initial
  adapter, which aborts immediately on transient permission/removal errors.
- `enumeration-green.log.gz`: the same tests pass after bounded retry; all seven
  native tests pass with ASan+UBSan. Fake USB avoids hardware effects. This does
  not instrument stock libusb or establish native platform acceptance.
- `android-build.log.gz`: offline wire tests and profile/optimized builds.
- `android-debug.log.gz`: ordinary debug build, which excludes the probe activity.
- `android-profile-tests.log.gz`: actual profile-source-set wire, worker-manifest
  and APK license-asset unit tests.
- `provenance.json`: tested source hashes, host executable hash and APK hashes
  plus probe inclusion inspection. Binaries remain local build products.

The owning review document describes architecture, exact commands, physical
interaction, recovery requirements and remaining production work. These synthetic
fixtures establish no USB/LAN performance comparison or production readiness.
