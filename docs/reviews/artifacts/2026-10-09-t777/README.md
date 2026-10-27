# T777 regression evidence

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The permanent malformed-number fixture ran against the old parser and failed
because `1tail` was accepted. Before the fix, preprocessing damage_regions.c
with `cc -E -DNDEBUG -Ihost/evdi` succeeded (exit 0). The retained normal Cargo
benchmark target now tests both contracts, covers invalid fields and a valid
scale-4 replay, and passes all five tests with ASan/UBSan where executed.
No production runtime changes; benchmark code remains coverage-exempt.
