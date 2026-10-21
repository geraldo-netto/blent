# Saved computers on Android

Open **Settings (⚙) → Saved computers**. Disconnect USB and wait for cleanup
before reading or changing the list. **Refresh saved computers** reads saved
trust without connecting a computer or enabling camera/microphone sharing.
Refresh keeps this panel open: the saved list and its status update, while the
Refresh and Close buttons stay available. They are actions, not another question.
Choose **Close saved computers** when finished reading the list.

Each entry shows its full computer fingerprint. Choose **Forget** for that
fingerprint, check the confirmation, then choose **Confirm forget computer**.
**Keep computer** cancels that confirmation. The next connection from a forgotten
computer requires approval. Forgetting does not change the separate camera/
microphone permissions or USB capacity settings.
The store rejects a confirmation if its saved revision changed in the meantime;
refresh and confirm the current entry again.

The list includes up to the store's existing 1024-peer limit. **No saved
computers** also covers a tablet whose identity has not been initialized;
browsing does not generate a key. Corrupt or mismatched identity/trust produces
an error and preserves stored data.

Accepted reads and forgets continue after closing the list or settings, and
across Activity recreation. Reopen the list to observe the result; an
unconfirmed selection must be
selected again. An expired view must be closed and reopened before accepting
commands. **Stop saved-computer access** requests cancellation, but a
forget already accepted by storage may have completed. Refresh after cleanup to
check the outcome.

While access is closing, conflicting operations remain disabled. A cleanup
warning means ownership is still unsettled; a timeout does not permit another
operation. Existing current-peer revocation and independent media controls keep
their own lifecycle rules.

Local automated UI/store/Binder checks are distinct from physical acceptance.
On2026-10-21, [T1252 selected-device acceptance](reviews/artifacts/2026-10-21-pairing-acceptance/T1252/README.md)
passes on the API36 RugKing Pad 2 Pro: independent reads after original worker
exit, Forget cancellation/confirmation, both denials, explicit remembered
approval, debugging-off persisted reconnect after source broker restart, and
original-peer revocation. A new independent read after revocation shows no saved
computers. Original tablet/helper exits precede debugging restoration; debugging
and existing power settings are restored. Pairing sends no media offers.

The denying endpoint ends its local setup; the other endpoint can remain in its
own approval/readiness window. In the selected host-denial run, the tablet's
unapproved dialog ends at its configured120000ms approval deadline. Local denial
does not promise immediate remote notification. Wait for each endpoint's cleanup
before reconnecting or accessing saved trust.

Earlier [T1237 evidence](reviews/artifacts/2026-10-21-t1237/saved-trust.json)
retains the initial persisted-list read. This selected-device evidence does not
qualify every store/provider or broader joined T846 behavior. API27/28 native
provider/accessory qualification remains separate. Camera T858, speakers T860
and microphone/duplex T859 have their own selected-device acceptance records.
