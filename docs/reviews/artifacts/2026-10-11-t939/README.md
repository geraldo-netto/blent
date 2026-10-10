# T939 bounded private camera schema

Typed camera consent, permission-result and actual consumer-retirement commands
share the existing bounded private control transport. Offers contain only validated
lease/profile/pending metadata; admitted routes additionally carry one locally
allocated port and private credential. Closure is exact-lease metadata. These
messages establish neither native camera permission nor original worker authority.

Canonical management profile validation and focused shared lease validation own
range checks. Strict boolean parsing rejects noncanonical values. Media tags are
separate from connection/trust tags and preserve existing display bytes. Binder
still enforces same UID, one-way delivery, positive process generation and bounded
Parcel/control sizes; encoded media never passes through it. Native runtime camera
commands remain unsupported until the retained adapter in T940.

Permanent tests cover all boolean combinations, profile/port boundaries, invalid
lease types, every truncation, versions/tags/trailing fields, 1,024 seeded command
and event mutations, and actual Robolectric Binder/Parcel delivery on API 27/34.
These are private transport fixtures, not physical Android camera or native USB
acceptance. Draft failures were test fixtures: credentials intentionally have
reference identity, and one-way Binder contains remote exceptions. Fixtures now
compare exact canonical bytes and explicit remote rejection. No product bug is
claimed from that draft log.

All 673 offline connection tests pass. All 65 scoped methods meet individual
native JaCoCo 80% executable-line coverage; all 17 authored constructors pass,
supplementing T848. All 77 reviewed functions have cyclomatic complexity at most
nine. Exact inventory, hashes, XML/execution counters and logs are retained.

T940 implements retained worker/runtime dispatch, T941 main-process presentation,
and T936 joined private integration. Hardware capture/proof and capability
composition remain T932; host adaptation T933 and physical acceptance T858.
No downloads, installation, reloads or device changes occurred.
