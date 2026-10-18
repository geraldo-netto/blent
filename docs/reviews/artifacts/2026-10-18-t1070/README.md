# T1070: intentional platform permission callbacks

Two SDK 27/34 Activity regressions intentionally invoke the framework permission
callback for camera grant and microphone denial. Narrow DEPRECATION annotations
and comments document those calls; all existing assertions and cleanup remain.
No production API, method body, metric suppression or new test is introduced.

The four retained permission cases pass. Offline test compilation emits no warnings.
Before/after logs and source hash are retained. Cached pinned Kotlin PSI 1.9.20
source audit covers 38 test functions/callbacks, maximum estimate 2; native
SonarQube remains T1019. Test annotations allocate no resource or change lifetime.
No network, installation, push or live device operation occurred.
