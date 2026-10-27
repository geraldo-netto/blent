# Fork name and attribution: local review

Historical record. Names, commands, support claims and TODO states below describe
the recorded revision. Use the [review map](README.md) for current guidance.

Reviewed 2026-10-10 at `e5a1d81`. The maintainer requested local files only and
clarified the question: whether renaming the fork and separating original and
new contribution notices is permissible. No online legal, trademark, patent or
publication verification was performed. This is a reading of the retained
license and implementation, not a jurisdiction-specific legal opinion.

The inherited [MIT license](../../LICENSE) expressly permits modification,
publication, distribution, sublicensing and sale, conditioned on including the
copyright and permission notice in copies or substantial portions. It contains
no requirement to preserve the project name or display upstream branding in the
interface. Renaming UScreen to Blent therefore does not conflict with that
license condition. This does not establish trademark clearance for a new name.

[COPYRIGHT.md](../../COPYRIGHT.md) correctly separates majmichu1's inherited
UScreen material from Geraldo Netto's original Blent contributions and records
the inherited commit. Moving, splitting or extending inherited code does not
transfer its copyright to the fork author. The amount of new functionality does
not remove the notice requirement for inherited material that remains.

Keeping the upstream notice in LICENSE and the new contribution notice in
COPYRIGHT.md is consistent with the local MIT text, provided both accompany
distributed copies as the project promises. That text does not require a header
in every source file. Third-party files with their own terms remain separately
attributed; this conclusion does not replace those terms.

## Verified evidence and original finding

T845 was subsequently fixed; see [APK notice packaging and validation](2026-10-10-t845-android-notices.md).
The findings below describe the cached APK inspected before that fix.

- LICENSE and `host/evdi/evdi_lib.h` remain byte-identical to inherited revision
  `96fce89c2b3e57e092d59a4890235aec9ae777c3`.
- Linux distribution documentation includes LICENSE and COPYRIGHT.md through
  `packaging/distribution-docs.txt` and `scripts/copy-distribution-docs.sh`.
- **T845:** the cached release APK contains neither project notice nor the MIT
  permission text. Its only notice-named ZIP entry is OkHttp's public-suffix
  NOTICE; scanning every decompressed entry also finds no original-author,
  fork-author, MIT permission or Apache license marker. Android Gradle has no
  project-notice asset generation. The publisher separately distributes the APK,
  so documentation in a Linux archive does not ensure APK recipients get it.
- Cached OkHttp 4.12.0, Kotlin stdlib 1.9.21 and coroutines-android 1.7.3 POMs
  identify Apache-2.0 licenses. The same APK scan finds no Apache license text.
  Full runtime dependency/NOTICE inventory remains part of T845; a POM alone
  does not establish all artifact-specific obligations.

[Local audit](artifacts/2026-10-10-fork-notices/local-audit.json) records hashes,
comparison results and APK marker searches. The APK hash identifies a cached
artifact; it is not proof that any public release contains the same bytes.

Fix T845 by packaging project and applicable dependency notices inside the APK,
with an accessible notice location and a normal automated artifact check. Before
changing packaging, prove the permanent regression fails on missing notices;
then verify release assembly preserves exact upstream attribution and license
texts. Do not replace original copyright with the fork author's notice.
