# T1237 USB reconnect transport fence

T1237 reconnect acceptance passed on the API36 RugKing Pad 2 Pro.
The later [T1238 comparison](../2026-10-21-media-acceptance/T1238/README.md)
verifies original camera/audio Stop retirement on the current selected-device build.

A cancelled discovery wrote a public greeting into the Android accessory queue.
A later logical close/open reused the same USB transport and read that old
message as its approval-ready marker. The owner reproduced
`Invalid approval-ready marker`; resetting only the selected physical tablet
restored authentication. See `reproduction.json` and the permanent regression's
failing-before `red.log`.

The native adapter now claims the old accessory interface before resetting it;
a foreign claim rejects without reset (`exclusive-claim-red.log` proves the
missing exclusion before the correction). It retains the old libusb device
reference across reset, closes its handle and clears endpoint metadata. Only
a different native device object at the same selected physical port admits the
replacement. Reset success alone, a still-visible old device, and unknown native
error results cannot admit a new link. Cold AOA startup remains independently
checked. No old marker scanning or permissive protocol path is added.

The adapter owns the original handle/claim and retained device reference until
its close and the replacement fence. The retained reference prevents pointer
reuse as a false freshness witness. Startup has no transfer threads. Successful
replacement releases the old reference and claims the new interface before
publication. On rejection or failed re-enumeration, helper process exit releases
remaining native resources; the broker retains the original helper admission
until its waitpid evidence. An unknown retirement never admits a replacement.

Verification uses 26 normal `test_aoa*.py` cases, GCC/gcov 13.3.0 native counters
and AddressSanitizer/UndefinedBehaviorSanitizer. The fixture independently models
queued messages, original/fresh objects, reference conservation, original claim
before reset, stale selection, transient enumeration, and native error bounds.
Its faulty control keeps the original object after reset and must fail closed.
All three changed production functions have 100% executable-line coverage:
`wait_fresh_device` 14/14, `reset_accessory` 15/15, `open_target` 22/22.
Counters, compiler commands, source hashes and logs are retained here.

Source syntax estimates cover 39 changed/new production, fixture and test
functions; maximum 8. These are not licensed SonarQube measurements. Native
SonarQube is unavailable offline. The full project coverage gate also retains
its separately tracked cached-grammar/platform obstacles; this evidence makes
no full-project pass claim.

The activated helper belongs to the source broker, with exact digest in
`activation.json`. The globally installed AppImage is unchanged. Native saved
computer trust was independently read after the previous Android connection
worker exited (`saved-trust.json`); an authenticated link alone was not treated
as proof of persistent trust. Debugging-off connection6370 delivered front360 frames, microphone PCM and an
owner-confirmed speaker tone. Exact-peer revocation retired all three original
host sessions (exit1, not clean-success) and removed their audio endpoints.
Without unplugging or ADB, the current helper reset the already-accessory tablet
and established fresh connection8535. Host approval used its exact fresh peer
ticket; saved tablet trust avoided another computer prompt. New explicit Android
Start actions delivered rear180 frames, microphone PCM and a second confirmed
tone. See `physical-acceptance.json` and the bounded retained evidence.

This trial validates this tablet, not every Android USB reset implementation.
T1238 subsequently verifies original media worker death after debugging-off hot
reconnect and local Stop, before debugging restoration. The earlier uninstrumented
report has no established cause; it does not invalidate the observed transport
fence or fresh authenticated media delivery. No images, raw recordings or private
pairing keys are retained.
