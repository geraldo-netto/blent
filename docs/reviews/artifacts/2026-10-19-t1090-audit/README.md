# T1090: legacy ADB mapping retirement prerequisite

A fake-ADB audit showed that listing then removing a mapping cannot prevent another client's intervening replacement. This was prerequisite evidence, not a production fix; the later exclusive-endpoint ownership decision and T1090 implementation supersede its blocked status.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

No production fix or native device validation is claimed. The current
`host/src/camera/bridge.rs::Bridge::close` reads `reverse --list`, checks the
remote/local pair, then executes a separate `reverse --remove REMOTE`. Audio
uses the same adapter. Original creator/command retention cannot make these
two native operations atomic.

Installed ADB 34.0.4-debian (1.0.41) documents removal by remote endpoint alone;
its installed help provides no expected destination or generation condition.
Only local help/version were executed. No server/device discovery, mapping,
installation, network transfer or tablet interaction was performed.

The retained `reproduce.py` replays the production command ordering against a
private fake executable in a temporary directory. Its listing returns the
original pair, then deterministically simulates another client's rebind before
removal. The remove operation deletes that foreign destination, as captured in
`result.json`. This is a command-sequence counterexample supporting static review;
it does not execute the Rust adapter and is not a permanent production regression
or proof of unrestricted native behavior. A real regression must be added and
shown failing before implementing a fix, then retained after it passes.

An app-local mutex or random remote port cannot by itself exclude another ADB
client, and retaining the original child proves command retirement rather than
mapping identity. Missing prerequisite: a validated native conditional-removal
mechanism, or an explicit ownership/exclusivity decision defining allowed
concurrent external mutations. T1090 remains unresolved/blocked; dependent camera
and audio availability integration must retain that dependency. Do not silently
assume external exclusion or count this audit as implemented cleanup.

The fake has one mapping, one removal receipt and two synchronously waited child
commands. Temporary executable/state ownership ends after both children exit;
there are no retained callbacks or native handles. Script functions are absent;
no per-function coverage or native SonarQube evidence is claimed for this probe.

</details>
