# T1044: production capture import regression

Historical evidence. Results apply to the recorded source and conditions. See the
[artifact guide](../README.md) for scope and terminology.

T1041 accidentally guarded the production `tokio::sync::watch` import with
`cfg(test)`. The test binary compiled, but the normal daemon failed with 17 E0433
errors in capture state and signatures. The normal Linux CI now permanently
checks the production daemon with accessory transport, without test-only imports.

The exact compile regression fails before removing the guard (`red.log.gz`) and
passes afterward (`green.log.gz`). Local execution sets `CARGO_NET_OFFLINE=true`
for the CI command `cargo check --locked -p blent --features accessory-transport
--bin blent`; no packages were transferred. Existing dead-code warnings concern
retained native ownership and activity integration tracked under T1015. No
production function behavior or test assertions changed; no new functions require
native line or complexity measurements. Existing CI release builds remain.
