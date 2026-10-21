# T1250 broker help follows implemented media integration

Native `blent-usb-broker --help` still described production media integration as
pending. T932/T933/T978/T998/T999 implement those adapters; T1238 now supplies
the selected API36 tablet's current-build native Stop/retirement evidence.

The Clap description now reads `Experimental per-user USB broker for paired
display and media`. The experimental label and scoped device/backend limitations
remain. Only this documentation string changes: no function, interface, option,
default, protocol, native owner or cleanup boundary changes.

The existing cached-SDK runner rebuilds the broker offline with locked
dependencies and the existing native-audio/accessory-transport features. The
rebuilt native `--help` exits0. Its first line has the corrected description;
every subsequent line exactly matches the retained before-help output, including
all options and defaults. Rustfmt with the crate's Rust2021 edition passes.
Compressed before/after/build output and exact source/executable hashes accompany
this note. The idle acceptance broker was stopped before rebuilding; installed
services are not replaced or restarted. No network transfer occurs.

This is a documentation correction, with no behavioral bug or new resource
lifetime. No artificial behavioral regression or new coverage/complexity result
is claimed. Existing CLI validation/invalid-boundary tests remain unchanged.
