# T943 portable capture target and native ownership

Portable capture claims remain owned through lens replacement and actual native retirement. Permission is checked around lazy construction; equal metadata cannot identify a replacement target.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Independent CameraCaptureTarget/Authority/Claim contracts carry validated local
metadata and original lifetime controls without wire, Binder, UI or OS types.
Each possible native request retains its own claim before lane admission. Actual
lane retirement/no-start releases that exact claim; execution ended, logical stop
and socket EOF cannot substitute. Lens replacements retain all outstanding claims;
discarded pending requests publish no-start without creating a native worker.

A lazy CameraCaptureRun checks live OS permission and the exact retained claim
both before and after constructing its lazy backend. Single execution and
retire-before-start gates reject invalid lifecycle sequences. Every created backend
remains reachable through actual retirement. Factory/admission failures cannot lose
an acquired claim. A failed actual worker exit keeps its claim quarantined and
prevents another native capture; other resources have independent lanes.

CameraBinding accepts an explicit portable target while retaining the latest ADB
invitation for fallback. Equal metadata is not target identity. Old workers cannot
change current status or close a replacement. Local stop/target loss close only the
original camera authority; cleanup proof follows actual native release. External
permission loss cannot use a legacy permission callback. Front/rear and background
controls remain; an unselected target stays available for explicit local selection.
Control callback failure cannot skip local native stop/retirement.

Prototype invalid repeated execution, retirement before execution, cancellation
cleanup, premature unselected-target closure and stale selected lens after refusal
have permanent fail-before-fix regressions. Their final assertions are proven red
before fixes, then green. Other permanent tests cover claim/permission loss across
lazy construction, three lens requests with a discarded pending factory, equal
metadata replacement, both source choices, failed worker exit, control failure,
background reattachment and guard failure between claim retention and admission.
128 seeded invalid ports and profile/token boundaries reject before authority or
native acquisition. Existing status-dispatch regression supplies failure-callback
coverage; it is retained rather than duplicated.

All 955 offline camera/connection/worker/resource/audio/display/status
tests pass. All 37 scoped production methods meet individual native JaCoCo 80%
executable-line coverage; all 4 authored constructors pass. All 56 reviewed
functions have cyclomatic complexity at most nine. Counters, inventory, source hashes
and red/green evidence are retained. This is scoped coverage, not project-wide.

Authorities and native workers are synthetic in these new binding fixtures. The
actual original-connection adapter/profile projection is T948, Activity/permission
handling T944 and joined capability enablement T932. Production USB capabilities
remain display-only; camera hardware/native USB acceptance stays T858. No downloads,
installation, live host/tablet changes or external dependency modifications.

</details>
