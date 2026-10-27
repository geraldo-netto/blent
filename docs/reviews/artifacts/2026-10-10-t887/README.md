# T887 Android USB selection, permission and opening

Android accessory selection excludes permission-gated serial metadata and rejects ambiguity. Permission and late-open regressions retain exact owners; hosted fixtures do not qualify a physical permission dialog or Binder termination.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

Portable policy keeps bounded metadata snapshots, canonical nonwrapping selection
epochs, exact unique resolution and stale-result handling independent of Android.
Native adapters recheck actual attachment and permission, own immutable permission
requests and retain descriptors/bridges through late completion and failed close.
Metadata never grants TLS or media authority.

Retained fail-before-fix regressions cover same-ticket permission replacement,
receiver-thread native cancellation after queue rejection, lost PendingIntent
ownership after failed cancellation, registration completing after close, a new
unowned token created while prior cancellation blocks, and permission-gated serial
access preventing selection before permission. All regressions remain in the
normal Android test suite. Initial test-fixture failures are not bug evidence.

The local cached Android14 framework bytecode verifies that `UsbSerialReader`
checks USB permission for ordinary apps targeting API29+. Android selection now
consistently excludes serial before and after consent; ambiguous public metadata
is rejected. The portable snapshot format still supports a serial field for
adapters that can provide one consistently. No dependency code was changed.

Final validation: 361 connection tests pass without failures or skips. All 47
inventoried production functions pass individual coverage; all 78 executable JVM
methods, including constructors/getters and compiled callbacks, have 100% line
coverage. All 89 reviewed functions have cyclomatic complexity at most 7.
These are line counters, not a claim that every execution path was exhausted.
Native registration taking effect before returning an error also has a permanent
retained-ownership regression.

Offline reproduction:

```
./android/gradlew -p android --offline --no-daemon \
  -I /absolute/path/to/blent/scripts/coverage/android.init.gradle \
  :app:testDebugUnitTest --tests 'com.blent.connection.*' :app:profileCoverage
```

The production factory remains unsupported until T854 supplies composition,
configured metadata/capacity, selection/detach gates and lifetime supervision.
T855 supplies pairing presentation and physical acceptance; production media
remains T857–T860. Robolectric API27/34 and native descriptor/loopback fixtures do
not establish physical USB/permission-dialog/Binder-kill acceptance. No downloads,
installations, live USB changes or network changes occurred.

</details>
