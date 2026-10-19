# T1106: camera probe composes the shared ADB command adapter

The opt-in camera feedback probe imports the authoritative production bridge.
That bridge now depends on the crate-root ADB command adapter, which the example
had not composed. The normal permanent compile test fails before the fix with
`unresolved import crate::adb_command`, then passes after adding the existing
adapter source to the Linux example root. No implementation or mutable state is
copied, no visibility is widened, and no physical capture is executed.

Five tests pass: the new normal-suite compile regression and four existing T1079
original-command lifetime/cancellation controls. All examples also compile with
installed tools using offline, locked Cargo. The three scoped Rust/Python source
function estimates are 1 (repository parser estimate, not native SonarQube).
No maintained production function body changes; there is no new runtime counter
requirement to manufacture for module composition. Existing tests remain intact.

The compile regression reuses the T1079 command sponsor. Its original process,
process group and log remain owned through actual compiler exit or explicit
cancellation/join; temporary log storage ends afterwards. No elapsed kill cap,
new native cleanup implementation, queue or device allocation is introduced.
Runtime ADB command/child/collector lifetime behavior remains the existing shared
adapter's responsibility. Known reverse-map ownership limitations remain T1090;
compilation does not resolve them or qualify native camera/tablet acceptance.
