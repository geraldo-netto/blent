# T1163 — API eligibility is not provider qualification

Retain supported() as the explicit adapter boundary: SDK>=27 is eligible;
SDK<27 is rejected. The application minSDK27 makes rejection unreachable through
normal installation, which explains ObsoleteSdkInt. The check also guards direct
adapter use before socket wrapping. Installed Kotlin/JDK bytecode retains both
outcomes with an integer comparison against27; no suppression or simplification
is needed to preserve this narrowly stated defense.

All six retained SecureConnectionTest cases pass offline. API27 uses TLS1.2 and
the authenticated preface without public ALPN; API34 exercises ALPN, mismatched
identity/protocol rejection, bounded malformed certificates, read-timeout
preservation and owned-socket closure. Existing oracles are unchanged. Below-min
eligibility is a static/bytecode result, not an executed SDK26 native test.

Native API27/28 provider, AndroidKeyStore and accessory qualification remains
blocked T863; hosted TLS must not be promoted to that missing evidence. Socket
ownership, cleanup supervision and actual-worker retirement are unchanged; no new
resource, identity, callback or lifetime is introduced. No physical or network
transfer action occurred.
