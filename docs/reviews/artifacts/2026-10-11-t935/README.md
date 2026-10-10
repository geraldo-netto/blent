# T935 consent-gated camera local endpoint

The existing local-resource group supports the camera's single uplink channel
without assuming display's two ports. Typed private metadata preserves the exact
lease, credential and camera profile. Opening and admission depend on an independent
permit port; the camera adapter rechecks current portable consent/OS choice, and
capture requires actual protocol admission. Default permission remains display-only.
Shared lease-shape validation preserves the existing display contract.

Permission loss or provider failure hides the camera route and closes only its
own native listener/pump. Logical closure retains handles and outstanding loans
until their actual retirement; a failed factory/stop remains quarantined. Monotonic
clock and block/profile/lease bounds reject invalid constructed inputs before
opening. Unopened groups cannot publish Prepared.

Three permanent regressions fail before their fixes: a camera endpoint could open
without an explicit permit; a provider exception escaped route publication; and an
unopened empty group published Prepared. The same tests pass in the final suite.
Additional cases cover bind/admission revocation, stale authority, endpoint/pump
failure, independent display operation, delayed cleanup and bounded invalid metadata.

The real JVM loopback fixture uses production camera management/mux/pumps and one
actual local TCP listener. A camera greeting crosses only the camera channel, its
feedback returns to that socket, and the port can be rebound after joined local
retirement. This is transport evidence, not hardware camera capture, real codec
decoding or physical Android/USB acceptance. The original display loopback and
all existing connection regressions remain in the normal suite.

All 650 offline connection tests pass. All 25 scoped production methods have
individual native JaCoCo line coverage of at least 90%; five authored constructors
also have 100%, explicitly supplementing T848. All 48 reviewed production/test
methods have cyclomatic complexity at most nine. Native execution/XML counters,
source inventory/hashes and red/green logs are retained.

T936 composes local controllers and private IPC publication/consent replies; T932
composes Activity/isolated hardware capture, including its separate actual-worker
retirement proof, and enables capabilities. Production remains display-only until
that integration. Joined physical camera acceptance remains T858. No download,
installation, reload or tablet trial occurred.
