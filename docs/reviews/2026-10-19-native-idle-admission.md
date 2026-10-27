# Original advice cannot qualify a replacement native reader

A replacement native reader pins and rejects advice that existed before its first source access. Each read and the original pin have explicit descriptor ownership, preventing stale advice from qualifying a reused source. Tests cover invalid input, exhaustion and descriptor reuse within a finite fixture scope; they do not establish unrestricted native safety.

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

<details>
<summary>Original findings, requirements and verification</summary>

T1120 adds a small Linux reader owner in `idle_control.h`. The FIFO writer begins
it before opening/accessing its first source, retaining an O_PATH/CLOEXEC pin
on the private regular advice object already present. Each bounded read opens
its own candidate descriptor, compares actual device/inode against that original
pin, and rejects the old object. A missing initial file permits future current
publications; other admission failures visibly retain compatible cadence.

Rust may allocate the passive replacement FIFO earlier. The only consumer of
its idle advice is this native writer, whose cutoff precedes its first source
access. Old native helpers/readers must still actually retire before replacement;
T814/T1028/T1036 process/lease fences are preserved. For a cooperating publisher
that remains live, T1117 retains its original source inode until request cleanup
settles, preventing source-inode reuse. After publisher loss, any residual advice
is present at the new native cutoff and rejected independently of source inode
reuse. Publications remain atomic replacements under T1114/T1118; an in-place
rewrite is not a current publication. Arbitrary noncooperating same-UID mutation
is outside this private-namespace ownership contract. The shared-ring path still
uses compatibility only; T1116 must adopt this owner before consuming advice.

One native worker exclusively owns the reader. Its tag distinguishes disabled
(no fd), admitted/no prior object (no fd), and admitted/pinned (one fd). begin
allocates locally, validates before handoff, closes on rejection, and preserves
an already live owner on repeated begin. Queries borrow only within that worker;
they retain no buffer/view/callback. Each query owns at most one additional read
descriptor, closed on every path. A candidate matches the original pin only if
both device and inode match; retaining the actual open original prevents inode
reuse. Unknown pin observations reject advice. No filename unlink or stale
snapshot removal occurs. Initial logical record size is capped at 159 bytes;
this is not a proof about arbitrary filesystem physical metadata/storage use.

The invariant is zero or one retained original descriptor, plus zero or one
transient read descriptor. No owned descriptor is released while any borrower
can still access it. Initialization starts disabled; successful begin creates
one of the two admitted states; queries preserve it; end closes the owned pin
once and resets to disabled before any future query/reuse. Zero initialization
and repeated end never close a foreign numeric descriptor. Native tests observe
the original fstat identity after unlink, actual close, actual numeric fd reuse,
and preservation of that new owner. No heap allocation or retaining cycle is
introduced. A copy of an owning reader is forbidden by its single-owner contract.

The writer settles its frame lease and last control access before end and native
return; the caller still joins that exact writer before releasing its surrounding
context/frame storage. Expiry, logical clear, reference counts and inactivity
never release the pin. If native IO/stop cannot finish, the pin remains owned and
funded, rather than reporting retirement. Eventual cleanup assumes native calls
return and the requested stop is scheduled; process exit releases its descriptors.
Request links remain owned by their publisher/private namespace; closing this
reader does not claim to remove them. A killed test parent can leave a private
temp directory for OS namespace cleanup; normal/error fixture paths retain the
parent TempDir until the original C child has returned, then remove its files.

[Native evidence](artifacts/2026-10-19-t1120/README.md): deterministic red/green,
retained boundary/interleaving/exhaustion/reuse cases, three faulty controls,
67 normal tests, individual counters and source audits. This covers the selected
single-reader fixture domains and current code refinement, not an unrestricted
native-safety theorem or device acceptance. T1115 still owns joined portable
viewer/liveness/recipe/decoder/ACK qualification; T1116 owns shared/native capture.

</details>
