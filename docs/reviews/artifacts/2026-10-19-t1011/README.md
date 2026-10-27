# T1011: joined Android display activity acceptance

Joined Android tests exercised pause, resume, stale routes and held decoder retirement through the real application components. The codec and sockets were simulated: all 2,234 tests passed, but physical rendering and suspend/resume remained unvalidated.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Permanent SDK27/34 acceptance joins the actual SessionCoordinator/control source,
RemoteDisplay, DisplayWorkerRun, VideoReceiver/DecoderSession, streaming power
binding and window binding. Its native boundaries are a memory-only socket,
bounded MediaCodec shadow and existing power fixture; no tablet or actual decode
is involved. The shared wake rig accepts an optional receiver, preserving the
existing default composition. No production implementation changes.

An initial paused source opens neither decoder nor wake owner. Resume authenticates
the original route and supplies fresh SPS/PPS configuration and an IDR-marked
access unit through the actual reader/decoder queue. Pause retires the actual
original receiver job and codec. A separately held simulated native fence blocks
replacement through 16 coalesced resumes and a route/credential change. Original
socket messages and rendered callbacks cannot revive stale authority. After the
held fence settles, exactly one fresh decoder authenticates the current route;
its empty queue requires newly supplied headers and IDR bytes. Background stop
withdraws window demand and invalidates the last render receipt. NAL markers prove
the handoff contract here; the shadow does not validate H.264 decoding, physical
rendering, process death or device suspend/resume. T914 retains physical acceptance.

Each fixture owns at most two codecs/workers, a 1 KiB input buffer per codec, at most
four copied input records, two 1 KiB pipes and its Surface/SurfaceTexture. Original
jobs are captured before stop clears VideoReceiver's current job reference. Pipes
close independently on cleanup; all original scope jobs and decoder retirement
join before the shared surface/texture releases. Held fences are explicitly
settled by the cleanup sponsor. Missing proof is tested as retained ownership,
never inferred from logical completion. Conditional cleanup assumes those original
fences eventually settle; a permanently stuck original remains funded and blocks
replacement. No physical native lifetime guarantee follows from these hosted tests.

The two new cases pass. The initial full collection exposed the separate API27
fixture error T1078; its exact failure is preserved with that item's permanent
red/green regression. After T1078, a fresh clean offline native JaCoCo collection
passes all 2,234 cases without skips/failures/errors. All source hashes match the
frozen collection. Current production sources match the retained T1061 scope;
the current coverage checker, including strict inline-body attribution, verifies
all 39 changed production functions/callbacks at 100% native line coverage. This
test-only item introduces no new production function. Raw native XML/exec and
class hashes are retained. Kotlin PSI 1.9.20 audits 117 test functions/callbacks,
maximum estimate 6; native SonarQube remains T1019. No network transfer, installation,
physical handling or battery-saving claim.

</details>
