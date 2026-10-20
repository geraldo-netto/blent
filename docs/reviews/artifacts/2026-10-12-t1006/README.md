# T1006: explicit offline Rust binary unit targets

The collector supports --bins, independently or with --lib. Its recorded Cargo
command retains exactly these target selections, locked/offline resolution, fresh
exclusive build trees and complete native input/source hashes. No arbitrary Cargo
argument passthrough or function exclusions are introduced.

Permanent command regression is red before the feature and green unchanged after.
CLI checks retain all four library/binary flag combinations; an actual dependency-free
offline Cargo fixture verifies library, binary and combined unit-target selection.
All 11 collector tests pass, including real Cargo clean guard, stale native mappings,
foreign object links, existing outputs, source mutation and failed/empty collections.
All 27 reviewed tool/test functions have complexity at most 8. Coverage tooling is
nonessential development tooling; no production-function coverage exemption is added.

Full Packaging notice/version integration tests remain unchanged and explicitly deferred
under T833. This selection does not claim those tests pass or remove their obstacle.
No downloads, installation, live host services, USB/ADB or tablet changes.
