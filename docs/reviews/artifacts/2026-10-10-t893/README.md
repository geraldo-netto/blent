# T893 — production Android pairing controls

MainActivity uses a process-owned lazy connection controller, a dedicated USB
settings section and an approval dialog independent of settings visibility.
Only application context is retained; Activity recreation/background does not
discard native ownership. Android dialog lifecycle hides pending approval in the
background and restores it on resume. Explicit revoke/disconnect remains available.

Fingerprints carry distinct Computer/Tablet labels, compared with the same labels
on the host. Remember resets per exact displayed choice; neither approval nor
checkbox state claims saved trust before a durable result. Deny and dialog Back
send explicit denial; no camera/microphone permission is granted. Small-window
content scrolls, and USB accessory intents use fresh admission instead of applying
unrelated ADB credentials. Duplicate attach does not replace an active attempt.

Permanent API 27/34 regressions first failed for accessory intents applying legacy
credentials and for inaccessible small-window approval content. Both now pass.
Activity recreation and background/resume tests confirm retained ownership and
current-choice presentation. A separate lazy-owner fixture exercises native
admission's no-accessory failure without a device. Production defaults remain
user-settings follow-up T898; offline saved-computer management remains T897.

711 offline tests pass without skips. All 139 scoped production functions and
162 raw executable JVM methods meet 80% individual line coverage. All 41 production
and 18 new test complexity entries are at most 9. Counters, source inventory,
results and red/green logs are retained. Broad validation also exposed the stale
T282 fixture, fixed independently in T899 without weakening assertions.

No installation, live USB mode change, download or physical pairing test occurred.
Host controls and joined acceptance remain T900/T901/T894/T855; production media
routing remains T857–T860. This is not a claim of working ADB-free media.
