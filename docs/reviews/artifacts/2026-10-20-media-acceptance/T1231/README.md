# T1231: explicit180p export from native360p capture

Actual tablet Camera2 metadata advertises640×360 but no320×180 PRIVATE surface.
The existing native-size rejection was correct. The180p webcam profile now has an
explicit360p capture projection and180p host export. USB uses that projection;
ADB now shares it instead of duplicating camera field projection. Other profiles
retain their selected native dimensions. Android's native-size check is unchanged.

Decoder max_pixels derives from the same checked capture contract, with existing
64-pixel alignment allowance. It is245760 for640×360 input. The output frame pool,
decoder transforms and V4L2 writers remain bound to320×180/86400 bytes. No decoder
allocation limit is removed or expanded to unrestricted native input. Invalid
profiles reject before native decoder admission or process creation.

All three permanent regressions failed first: independent capture/export schema
expectations, actual360p H.264 decode into180p output and ADB invitation fields.
Final normal native camera/config cohorts pass135+20 tests without failures or
ignores. Real FFmpeg performs the encode/decode regression; prior invalid-input,
transport, ownership, death, original-retirement and camera cases remain intact.
Fresh LLVM counters measure all four changed functions individually at100%:
invitation_profile17/17, Bridge::invite17/17, decoder command45/45 and connection27/27.
Raw LCOV, executed object/source hashes and red/green logs are retained.
Rustc1.98.1/cargo-llvm-cov0.8.7. Source complexity estimates5/2/1/3; native Sonar
unavailable, no tool download. These are source estimates, not Sonar evidence.

The rebuilt production host streamed actual front-camera360p at15 FPS with a
512 kbit/s target and rear-camera180p at15 FPS with a256 kbit/s target over the
authenticated direct USB route.12 samples at each size were read from the actual
V4L2 outputs;11/12 and12/12 frame hashes differ. No image content is retained.
The original speaker session stayed active; desktop360p/180p restart checks are
under T1230. T858 retains remaining debugging-off and consent/lifecycle acceptance.

Projection borrows the immutable original profile and returns a bounded value;
it owns no native resource or callback. Command construction validates before
admitting the original decoder obligation. Existing OwnedChild, frame pool,
original camera consumer, endpoint and V4L2 writer retain their exact ownership,
stop, quiescence and reuse boundaries. Camera-size choice changes no release
authority. Native errors, partial startup, cancellation, resource exhaustion and
unknown exit retain existing cleanup sponsors and funded capacity. Native/model
fixtures complement the physical sample; they do not prove unrestricted driver
lifetime safety, all devices, frame continuity or internet bandwidth savings.
