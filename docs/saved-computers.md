# Saved computers on Android

Open **Settings (⚙) → Saved computers**. Disconnect USB and wait for cleanup
before reading or changing the list. **Refresh saved computers** reads saved
trust without connecting a computer or enabling camera/microphone sharing.

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
On the API36 RugKing Pad 2 Pro, the saved-computer list was read after the
previous connection worker exited, and a new debugging-off USB connection
authenticated using that saved trust without another computer approval.
[T1237 evidence](reviews/artifacts/2026-10-21-t1237/saved-trust.json) records
the independent list read. This does not qualify every device's store/provider,
the native Forget flow, or remaining pairing/joined acceptance under T855/T846.
Camera acceptance T858 subsequently passes on this selected API36 device;
API27/28 provider/accessory qualification remains separate. The remaining T855
pairing checks retain their explicit pause.
