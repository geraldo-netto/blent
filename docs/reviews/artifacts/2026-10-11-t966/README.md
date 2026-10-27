# T966: no executable null-only initialization

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

Permanent source/native compiler regressions fail before the inventory correction.
Actual pinned compiler bytes prove default-null properties emit no PUTSTATIC
assignment; only generated singleton allocation remains. Current application
bytecode for AudioOwner, RemoteMetadata and ConnectionOwner likewise contains only
INSTANCE/Compose stability setup in the static method; JaCoCo removes that generated
method, explaining the missing native rows. No authored runtime operation is removed.

Empty init blocks and parenthesized literal null defaults are not executable bodies.
Calls, delegated fields, primary parameter defaults and nonempty init remain owned
and require counters. Missing measurement still fails; no counter is erased or
threshold lowered. Sixty-five bounded parentheses shapes preserve every call while
omitting only no-work null property declarations. All normal tool tests pass, named
review scores <=9. T965 retains the complexity inventory gap for other method kinds.

Existing unchanged T964 native counters now contain 2,480 Android functions and
only the two real default-constructor coverage gaps T967/T968. The application
was not rebuilt or installed for this tool-only correction. No device or download.
