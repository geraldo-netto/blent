# T1053: feature boundaries without dead-code diagnostics

`media_connection` has production consumers only in Linux accessory-enabled
camera/audio adapters. `media_request::validate_original` serves that same native
boundary. Both remain compiled for all tests, retaining their permanent portable
regressions. `EncoderProcess::adopt` serves CLI production and both feature sets'
native retirement tests; production in-process encoding uses its independent
worker path. The cfg conditions now express those actual consumers. No warning
suppression, test removal, allocation or runtime state transition was added.

Offline default and accessory-enabled library/binary checks pass without warnings.
The installed network-disabled Freedesktop SDK 25.08 production in-process build
also passes without warnings. Default Bridge and request suites pass four tests
each. Fresh native LLVM coverage uses the existing original-request and encoder
pause/retirement regressions; all four request and seven encoder tests pass, and both cfg-gated functions
measure 100% executable-line coverage separately.
The scoped repository source audit reports complexity 2 and 1 respectively; native
SonarQube availability remains T1019. Bodies, owners, cleanup sponsors, original
identity fences and storage bounds are unchanged. No new behavioral bug is claimed
and no artificial regression was added solely for compilation-scope maintenance.
