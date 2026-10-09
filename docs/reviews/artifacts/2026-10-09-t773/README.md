# T773 — metadata-cache decision

Evaluation closed: keep the pre-copy and post-copy metadata queries. No cache
is shipped. The owner's condition was a speed improvement without correctness
loss; the available invalidation mechanism does not meet it. This is a rejected
optimization, not a claim that metadata-query cost was reduced.

Five private-Xvfb trials (256 metadata pairs each) reproduce the cost: median of
trial medians 108.080 us for the EDID path, 10 requests per pair; named-output
benchmark path 88.803 us, 8 requests. Neither measures physical GPU/driver latency.
The ten requests are current resources, output info, EDID property, transform
and CRTC info, twice. They inspect mutable state and protect different sides of
the queued copy. Successful XInternAtom lookups already use Xlib's per-Display
cache: another 256 EDID atom lookups issue **zero** requests. An extra Blent atom
cache would save no round trip. The named-output case has no EDID property/atom;
its deliberately repeated failed atom lookups issue 256 requests, outside the
measured geometry pair and outside the named-output production path.

`events.c` subscribes to XDamage and cursor events, not RandR metadata changes;
periodic capture skips this event setup. Its 256-event processing budget cannot
be treated as a metadata-generation barrier. A same-geometry EDID replacement
is also not identified by pixel damage or output dimensions. No cache based on
those signals is justified. A future pipelined-query implementation would be a
separate proposal with its own protocol, dependency and native validation costs.

Permanent GPU lifecycle tests now cover ten transitions on each side of the
copy, both with no event backlog and 513 queued events (40 rejection cases):
EDID changes with unchanged geometry, changed CRTC ownership, disconnection,
missing CRTC, rotation, x/y movement, width/height change and transform change.
Each starts with a successful captured/released frame so a cache could have been
populated. Three fresh connections additionally exercise reconnect isolation.
The existing periodic and damage-cadence lifecycle cases remain.

These tests use a real private X server with wrapped boundary replies; they
verify Blent's rejection behavior, not physical hotplug or atomic snapshots of
arbitrary external server state. Negative controls in temporary copies removed
only the pre-check or only the post-check. Both fail the permanent regression
(`negative-pre.log`, `negative-post.log`). Production C was never changed.

The normal GPU coverage gate passes ASan/UBSan and all **52/52** maintained GPU
functions reach >=80% individual executable-line coverage. Reports and source
fingerprints are retained. No new exclusion; changed test/benchmark functions
meet the cyclomatic <=9 limit. No downloads, external code edits or live display
changes.

Reproduce with an installed/cached SDK:

```
python scripts/coverage/gpu_check.py /fresh/gpu-coverage
```

`PKG_CONFIG_PATH`, `LIBRARY_PATH` and `GPU_CFLAGS` may point to a private cached
SDK, as this run did. The request-count probe is
`scripts/benchmarks/c-review-gpu.c`; its build dependencies and isolated Xvfb
invocation match `scripts/benchmarks/c-review-performance.py`.
