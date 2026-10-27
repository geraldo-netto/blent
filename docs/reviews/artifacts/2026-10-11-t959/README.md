# T959: permanent worker probe contract compilation

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

The exact retained worker fixture is now part of normal Android test compilation,
never production APK sources. Original caller fails that permanent compile gate:
String is not DisplayRoutes. Use the explicit ADB route adapter, retaining fixture
token, local ports, revision and decoder format; all native ownership assertions
remain unchanged. Normal worker contract/manifest/service tests pass without skips.
The four reviewed functions remain <=9. Benchmark/native device trials are not
claimed; this is compile-contract validation. Experimental coroutine declaration
warnings discovered in this fixture remain tracked separately as T960.
