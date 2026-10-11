# T938: bounded camera frame sizes

Original boundary regression fails in debug (overflow panic) and release (wrapped
size). Invalid black-frame dimensions also panic before the fix. The permanent
regressions now reject invalid dimensions without allocation. The 225 bounded
width/height combinations either reject or return exact YUV420 byte sizes.

`CameraProfile.frame_bytes` is fallible and validates only supported even image
dimensions; unrelated codec/control options do not determine layout. Native pool
and blank-frame callers propagate errors. Luma size derives from validated frame
size, avoiding another unchecked product. Existing camera fixtures retain their
assertions with explicit successful results.

11 common camera tests pass in debug and release; all 30 host camera fixtures
pass. Individual native counters pass for all 16 production functions, exported
only from the two just-executed SHA-attested test binaries (T880 remains open for
normal reused-target collection). All 43 reviewed functions have complexity <=9.
No device trial, installation, download or EVDI change occurred.

Workspace all-features checking stops at missing libpipewire-0.3.pc, already
tracked under T835. Default camera code and its current callers compile and pass;
no all-features native support or unavailable coverage is claimed.
