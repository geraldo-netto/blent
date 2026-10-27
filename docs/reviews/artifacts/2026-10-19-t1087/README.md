# T1087: compose private batch code only with its native caller

Portable builds exclude private batch code whose only caller is the Linux runtime. Offline compiler checks passed; public contracts and function bodies stayed unchanged.

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

<details>
<summary>Recorded evidence and technical details</summary>

The private tablets::batch module is used only by the Linux platform runtime.
Its module inclusion now matches that caller's target/feature condition. Portable
no-default-features builds keep their current public contracts without compiling
unused native snapshot adapter code. No function body, schema, validation rule,
public interface, warning suppression or native implementation changed.

Permanent Python compiler cases invoke installed offline cargo rustc for portable
and default common configurations, denying dead_code in the maintained crate.
The portable case fails before the change on validate/deserialize/BatchVisitor;
the same case and default compilation pass afterward. The test imports the
existing command owner by module alias, avoiding accidental TLS fixture discovery.
That helper owns the original child/group, waits without a build-time kill cap,
and handles cancellation before releasing the funded temporary log/context.

Existing native snapshot/batch regressions and all 12 architecture checks pass.
No new production functions or resource transitions exist. Existing batch input
bounds, stack borrows and output ownership are unchanged; no per-function counter
or artificial runtime test was added for the module declaration. The three test
methods have source estimates 1 each (repository Python AST; version recorded).
Native SonarQube remains unavailable under T1019; no downloads were attempted.
This Linux evidence does not establish native Windows validation.

</details>
