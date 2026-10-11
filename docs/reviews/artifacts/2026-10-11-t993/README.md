# T993: validate native audio helper arguments

The helper boundary validates AudioProfile and the independent strict 64-byte
lowercase-hex media credential before label slicing or native child creation.
Permanent regressions first failed with an out-of-bounds slice panic and invalid
buffer profiles reaching native spawn. The same tests pass after validation.
Bounded invalid lengths 0–128 and all invalid byte values, including multibyte
UTF-8 boundaries, reject without creating a helper. Existing valid startup,
clocking and retirement regressions remain unchanged.

Fresh offline native collection: 26 host and 49 common audio tests pass.
All three affected production functions exceed 80% individually (92.5%,
85.71%, 100%); no missing counters. Changed Rust complexity: seven functions,
maximum six. Compressed red/green logs, native counters, source inventories,
LCOV and attestation retain the evidence. Collection used an exclusively owned
fresh target and immutable inputs. No downloads, real PipeWire devices or live
host/tablet trials. SDK-wide qualification remains T835.

Build-input attestation describes the tested working tree, including the
pre-existing uncommitted TLS fixture. Those unrelated edits remain unstaged;
this commit neither changes nor includes them.
