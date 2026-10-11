# T949 portable camera consent and SDK prompt lifetimes

Independent contracts represent explicit local camera enable, foreground presence,
fresh OS permission and separate background consent. CameraAccessOffer has no
credential, transport DTO or OS type; its opaque original-owner identity retains
intent across offer/permission/admission publications, never a replacement.
Exact current publication authority guards every consent and permission command.

CameraConsentPolicy starts only after explicit local foreground selection, samples
permission on every effect and allows background only with both original request
and local consent plus fresh permission. Foreground loss closes unapproved or
pending capture. Permission loss closes admitted capture, and background permission
loss explicitly closes preparing consent rather than awaiting an unavailable SDK
prompt. Permission callback booleans cannot substitute for a fresh sample. Denial,
rejected/throwing controls or unavailable launch disable local intent; source
replacement clears camera/background choices without affecting another resource.

CameraPermissionPrompts owns one attached launcher and one pending action. Logical
SDK cancellation never implies native capture retirement. Opaque attachment,
action and prompt identity reject forged, duplicate, stale and cancelled results.
Old Activity detach cannot clear a replacement launcher. Detached callbacks are
released and prompt actions must not capture Activities. Reentrant synchronous
SDK delivery consumes the original prompt before callback execution; failed launch
cannot retain or replay it. The SDK adapter must separately retain its original
outstanding result and refuse overlap even after logical cancellation.

Permanent regressions were proven red before fixing the prototype's replacement
attachment result, background preparing-permission loss and discarded prompt slot
stall. Tests preserve exact original intent under synchronous publication and
callback, grant/denial with changed permission, old prompt/source/attachment,
missing or throwing launch, rejected/throwing authority and cancelled/forged/duplicate
results. A complete 32-case bounded Boolean lifecycle property checks foreground,
background request/consent, permission and later revocation; incompatible admitted
and permission-required offer flags reject on construction. The initial property
incorrectly treated a permission-revocation control as permission to capture; the
final property checks current intent and explicitly proves preparing-background
closure red before the fix without weakening native denial expectations.

985 offline connection/camera/worker/resource/audio/display/status tests pass,
with no skips. All 30 scoped production methods meet individual native JaCoCo
80% executable-line coverage, all 9 authored constructors pass, and all 53
reviewed functions meet cyclomatic complexity at most nine. Inventory, raw/scoped
counters, source hashes and red/green evidence are retained. This is scoped
coverage, not project-wide coverage or Android hardware qualification.

Portable policy is not Android integration: T944 still connects SDK/Activity,
permission-change observation and original USB publication adapters; it also
corrects the existing bare-boolean MainActivity legacy permission forwarding and
unconditional old-Activity launcher detach. T932 composes isolated native capture
and enables the USB camera capability; T858/T863 retain actual hardware/accessory
and native API 27/28 acceptance. USB capability mask remains display-only. No
resource downloads, installation/reload, live host/tablet changes or external
dependency changes.
