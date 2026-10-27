# Measure idle connection work on a hosted JVM

This tool measures how much work the connection actors do while idle, using a
hosted Java runtime and synthetic owners. It does not measure Android battery use.

## Run

Run from the repository root with the installed Android SDK, cached Gradle
dependencies and a JDK exposing thread CPU and optional allocation counters:

```sh
python3 scripts/benchmarks/android-connection-idle/run.py /tmp/blent-idle
python3 scripts/benchmarks/android-connection-idle/run.py --allocations /tmp/blent-deadline-allocations
```

Both runs use Gradle `--offline`. Neither opens a tablet, USB transport, camera,
audio device, network connection or application installation. The opt-in init
script adds benchmark sources to the hosted unit-test compilation only for this
invocation; ordinary application and test builds retain their existing inputs.
The permanent Python tool regressions exercise task registration and compilation.

## What the experiment owns

The actor experiment runs the production worker and admission actors with
synthetic idle setup/permission owners. Each owns one original executor thread,
scope, actor, conflated wake channel and at most one outstanding cue/receipt.
Cleanup cancels and awaits the original execution, requests original retirement,
closes its dispatcher and joins its exact thread before replacement or return.
No timeout or elapsed process budget substitutes for those joins. Counter
reflection uses the installed JVM's public management interfaces; these borrowed
beans remain owned by the JVM. Missing CPU counters fail the experiment; missing
allocation counters remain absent rather than becoming zero.

## Measurement domains

Seven trials compare known-deadline fallbacks in order 10/100/1000/1000/100/10 ms,
then an unknown-capability 1000 ms control. Every trial has a one-second warmup
and a three-second measurement domain. Actor CPU, allocations and advance counts
for **both** actors are frozen before either actor's latency samples. Each actor
then receives 32 hinted changes and two deliberately missed hints. Receipt
identities must match their original cue. These finite domains bound the claim;
they are not execution kill deadlines. The lost-hint experiment follows a
three-millisecond gap, so observed fallback latency is slightly below its limit.

The separate allocation experiment compares `ConnectionDeadline.at(null)` and
its idle `combine` result, with consumption confined to one thread versus an
explicit escaping reference. Each case warms up one million iterations and
measures two million, retaining at most one escaped snapshot and clearing it on
every exit. The checksum and snapshot assertions preserve the stated work.
This exposes JVM scalar replacement and an escaping control; it does not assert
that Android ART makes the same optimization or attribute all actor allocation
to deadline snapshots.

## Read the evidence

Each output directory retains raw JSON, source hashes, the parent Git revision
and the Gradle log. Inputs must match before and after a run. Whole-process CPU
includes JVM compilation/GC and has coarser resolution than thread counters.
Hosted CPU, wake and allocation measurements establish neither native Android
idle/battery results nor complete media performance. Retain those limits when
comparing results or changing defaults.
