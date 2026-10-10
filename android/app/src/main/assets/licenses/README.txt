Blent Android distribution notices

LICENSE retains the original UScreen MIT copyright and permission notice.
COPYRIGHT.md distinguishes original Blent contributions from inherited code.
Its cross-project third-party references concern host components as well;
this Android inventory describes the dependencies packaged in this APK.

runtime-dependencies.json records the reviewed release runtime dependency
coordinates, artifact hashes and cached POM license provenance. Apache-2.0.txt
contains their Apache License, Version 2.0. Metadata-only modules have no
artifact, and R8 can remove unused runtime code. Upstream binary artifacts,
including classes.jar and libs/*.jar inside AARs, were checked for additional
LICENSE, COPYING, COPYRIGHT and NOTICE files.

OkHttp's public suffix data is separately licensed under MPL-2.0.txt.
publicsuffix-NOTICE.txt preserves its upstream notice and source location:
https://publicsuffix.org/list/public_suffix_list.dat

These notices are packaged at assets/licenses/ in every APK variant. An APK
is a ZIP archive; its notices can be read without installing or running it.
No license here changes the terms of other components or claims their authorship.
