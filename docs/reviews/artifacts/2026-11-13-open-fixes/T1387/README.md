# T1387 — independent saved-Wi-Fi reconnect

The stalled-Wi-Fi/healthy-USB regression fails before and passes after the fix.
The selected USB suite passes 37/37 tests. Four changed production functions
measure 100% executable-line coverage and complexity at most 7 (cargo-llvm-cov
0.8.7; project syn 2.0.119 frontend). The existing saved-address regression waits
for asynchronous discovery while retaining its network/credential assertions.

One scheduler slot owns one reconnect job across polls. Repeated polls cannot
queue duplicates. Poll cancellation leaves that job owned; stop/drop cancels it.
The existing Attempt remains the authoritative native creator/address owner and
sponsors compensation after caller loss. Tests release a delayed original creator
only after cancellation and observe route removal and original native lease drop.
Successful saved connections remain owned by the existing Wi-Fi policy. No change
to native adapters, resource funding or original-thread settlement is introduced.
Linux fixtures validate shared policy; Windows native acceptance remains T1382.
