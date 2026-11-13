# T1405 native attachment boundary

The application process retains one detach receiver and one freshness epoch across
Activity destruction, Disconnect, Deny and Revoke. An Activity action or accessory
extra only requests admission; neither creates an epoch. Admission requires a
current bounded native inventory containing one supported accessory, then consumes
the original epoch. Manual Connect consumes automatic freshness without requiring it.
Only a protected native detach notification renews freshness. Missing notification
evidence fails closed; manual Connect remains available.

One snapshot slot is retained through cancellation and original native return.
Epoch replacement prevents old snapshot completion from crossing manual cutoff or
detach. No accessory descriptor is opened here; existing connection admission,
permission, TLS and original-worker retirement remain separate requirements.
The process receiver is intentionally retained for the application lifetime.
Registration failure retains the owner and rejects automatic admission.

Native contract references checked on 2026-11-13:
- Android activity launch on accessory attach: https://android.googlesource.com/platform/frameworks/base/+/ca1943b2c3c99308185c31044db6156c9313442d/services/usb/java/com/android/server/usb/UsbProfileGroupSettingsManager.java
- Protected detach broadcast: https://android.googlesource.com/platform/frameworks/base/+/android12-release/core/res/AndroidManifest.xml

Robolectric API27/API34 fixtures exercise the native boundary and a blocked native
snapshot. They are not physical-device observations or new platform support claims.
