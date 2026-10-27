# T850 management record contract

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Five Rust and five Android tests pass offline, using the same independent
20 valid and 38 invalid binary fixtures. Permanent tests cover every record split,
coalesced records, truncated input, poisoned/ended decoder reuse, invalid scalar
ranges, credential ownership and 4,096 bounded mutation cases in each language.
Rust additionally verifies invalid models do not change caller-owned output.

The project coverage gate passes all 28 Rust and 24 Kotlin production functions.
Native Kotlin supplementary counters include 75 methods/constructors, all at least
80%; Rust per-function counters have minimum 91.67%. Complexity maxima are 9
(Rust production/tests) and 7 (Kotlin production). Source hashes, native counters,
manifest and logs are retained here. No device/media acceptance is claimed.

This implements parsing and serialization, not authority or transaction ordering;
T851 owns that policy. The initial Kotlin test attempted to treat one feed as the
whole input; it was corrected to consume each returned remainder before EOF,
retaining the trailing-byte rejection assertion. No production workaround or
regression weakening was needed.
