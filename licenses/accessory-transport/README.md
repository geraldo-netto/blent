# Optional accessory transport dependency notices

`inventory.json` records the 26 newly locked packages introduced while developing
T846. It is an incremental notice inventory, not a complete project dependency
audit. All material was read from the local Cargo cache, without downloads.
Package archive checksums match `Cargo.lock`; each copied notice has its own hash.

Original notice files are preserved verbatim, including ring's BoringSSL and
other-bits notices. `asn1-rs-impl` declares `MIT/Apache-2.0` but its cached archive
contains no license file; that entry records its declared authors and includes
the standard Apache-2.0 text from the adjacent cached asn1-rs package. The manifest
explicitly records this provenance rather than attributing a missing file to it.

Normal distribution documentation already includes the complete `licenses/`
directory. This inventory does not enable the experimental transport or establish
native backend acceptance. Review it whenever the optional dependency graph changes.
