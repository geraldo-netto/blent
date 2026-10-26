# T1269: current authentication guidance

Removed the pre-1.1.0 app compatibility rationale from the desktop security
control and the independent FileConfig schema documentation. Both now describe
current token-authentication behavior and recommend keeping authentication on.

The opt-out itself was not established to be compatibility-only: Linux has an
explicit configured authentication policy (`attachment/auth.rs`, including
permanent Disabled-policy tests, and `linux_main.rs::create_session_token`).
Windows explicitly rejects the disabled policy. This documentation correction
does not alter authentication, pairing, configuration, native resources or
platform support. No artificial behavior test is needed for a prose correction.

The normal native Linux GUI unit suite passes all 116 tests, offline/locked,
with the installed SDK runner. Existing security/settings tests remain intact.
The GUI startup integration target is outside this recorded unit run. No native
Windows runtime acceptance is claimed; its preview labels remain unchanged.
