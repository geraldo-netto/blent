# T1212: offline native host test environment

The existing host has real D-Bus, private Xvfb, PipeWire and namespace tools, plus
cached SDK25.08 headers/libraries and its matching codecs-extra extension. The
new optional development runner selects those original inputs explicitly. It
performs no installation, fetch, synchronization or global loader override.

The original SDK build lacked the multiarch header sysroot. Runtime then exposed
transitive RPATH, private PulseAudio library and SDK base codec limitations.
Matching cached codecs-extra supplies libx264. Real host PipeWire tools clear only
the SDK plugin overrides at their environment boundary; SDK-linked Rust adapters
use the SDK plugins. These adapters execute exact original installed tools and
never manufacture native results. A standard target-specific Cargo flag also
preserves the selected ABI in generated subordinate benchmark builds when
cargo-llvm-cov removes global instrumentation flags. Missing prerequisites reject.

All9 permanent runner controls pass, including offline/locked command policy,
argument boundaries, matching codec versions, foreign prefixes/tools, missing
originals, immutable concurrent publication and staged-file cleanup on error.
They are wired into the normal host tooling test target, whose new test passes.
Retained deliberate missing-RPATH/Pulse/subordinate controls fail before repair.

Native verification:1007 library cases,392 current CLI cases and11 audio cases
pass (five preexisting ignores remain). Actual private-Xvfb activity2, native
PipeWire/audio transport2, namespace lifecycle2, generated shared-encode replay1
and Wi-Fi setup3 pass. The generated benchmark compiles actual source and
encodes/decodes real frames. T835 retains complete-suite acceptance behind the
excluded native distribution fixture; current supported-package checks and the
latest tooling outcome are recorded under T833.

Native LLVM counters measure every one of the64 maintained FFmpeg/PipeWire
functions individually at85.714–100%. Direct export uses exact executed objects:
unchanged original library/audio/native objects plus the refreshed current CLI
test object after T1214. Current source/object hashes and raw LCOV are retained.
Rustc1.98.1/cargo-llvm-cov0.8.7. This establishes the selected cached native ABI,
not unrestricted GPU, old Android provider, portable-release or physical media
support. No production function changes in this development-runner deliverable;
nonessential verification scripts/test fixtures retain their coverage exemption.
Source-model function complexity is1–7; native Sonar remains unavailable.

Ownership: the caller selects an owned target cache; one immutable SDK symlink
and byte-identical original-tool adapters may be reused. Foreign prefixes,
symlinks and different content reject. Publication writes one bounded tool script
to an owned temporary file and atomically links without overwriting a winner;
context cleanup closes/unlinks the staged file on success, fault or rejection.
Original executables and cached SDK files are borrowed for the entire run. Tool
argument forwarding preserves caller bytes. The runner replaces itself with Cargo,
retaining the original foreground process group and caller's cancellation owner.
Native fixtures retain their existing original-process/namespace cleanup owners;
this script neither infers native death from logical status nor changes them.
The owned cache remains funded until those original processes actually retire;
the caller may remove it afterward. No background synchronization, live tablet,
service restart, EVDI/Windows implementation change or transfer occurred.

The historical unfiltered all-features baseline completed all45 test targets:
1640 passed, three failed and five existing ignores. Its target-by-target numeric
summary remains. Subsequent packaging maintenance has a separate current tooling
result under T833; neither record claims a complete-suite pass. Raw native LLVM
counter/refresh/replay evidence remains; obsolete supporting records were removed
intact during the owner-directed reference cleanup.
