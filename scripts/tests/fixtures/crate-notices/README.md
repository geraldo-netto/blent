# T1280 metadata fixture

`accesskit/` contains only the normalized Cargo.toml and .cargo_vcs_info.json
from the published accesskit 0.19.0 crate. Its archive SHA-256 is
`e25ae84c0260bdf5df07796d7cc4882460de26a2b406ec0e6c42461a723b271b`.
The metadata-only fixture exercises the exact manifest/VCS pin without requiring
a local Cargo cache or network. It is not an executable crate fixture.

AccessKit's upstream MIT and Apache-2.0 texts, revision and immutable source URLs
are retained under `packaging/appimage/supplemental-notices/accesskit/` and its
catalog. Production-source validation uses the full cached registry archive,
as recorded in the T1280 review evidence.
