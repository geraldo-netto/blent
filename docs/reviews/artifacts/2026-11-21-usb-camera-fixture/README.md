# T1474: direct-USB camera fixture, 2026-11-21

The accessory-enabled library suite did not compile: its retained camera TLS
fixture omitted Header.capabilities after BLCAM003 was introduced. Its byte
fixture also omitted the 20-byte capability record, 9-byte frame receipt and
18-byte live-control message. Updated the current independent fixture and its
round-trip assertions; retained the existing delayed-revocation/retirement checks.
No production protocol fallback or old format is added.

Before: accessory-enabled compilation failed at the missing Header field.
After: all 249 selected broker tests passed, including the actual private-TLS
camera registration test. This is a software transport/retirement regression,
not physical direct-USB hardware acceptance. Changed test functions stay <=9
using the project Rust syn 2.0.119 complexity checker.

The fixture retains a bounded 28-byte feedback buffer (OK plus one 26-byte
feedback record); original socket, peer task, broker task, registered route,
release signal and cleanup remain owned until the existing exact joins settle.
