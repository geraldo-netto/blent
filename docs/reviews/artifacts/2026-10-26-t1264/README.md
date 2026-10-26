# T1264: preserve exact ADB endpoints in native audio fixtures

Two new permanent Python regressions fail before the fixture correction: reverse
serialization changes the requested device port, and both tablets connect to
that device port instead of its host destination. The normal audio_native Cargo
target invokes these regressions. It now passes all three tests, including both
original real private PipeWire PCM/clocked/unclocked and retirement scripts.
The fake ADB rejects rebinding and mismatched removal instead of discarding a
foreign route. The original independent audio assertions remain intact.

Only test fixtures changed. Bounded mapping state belongs to the private temporary
directory; mock socket interception prevents external access in the new tests.
The original native fixtures retain their own process/thread cleanup. No physical
ADB, microphone, speaker or tablet is accessed. No production coverage changed.
All measured test functions meet complexity <=9 (local syn 2.0.119/Python 3.12.3).
The SDK, codecs and command are retained in green.log.gz; dependencies were cached.
