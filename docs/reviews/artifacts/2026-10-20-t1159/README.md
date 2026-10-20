# T1159 — codec constants on older supported SDKs

The InlinedApi warnings identify numeric profile vocabulary, not calls to newer
platform APIs. Installed Kotlin1.9.20/JDK21 bytecode emits integer 1/2 for AV1 main
profiles and integer8192 for HEVC Main10 HDR10Plus. Neither method resolves the
newer constant field at runtime. The older CodecProfileLevel/profileLevels types
remain ordinary supported-SDK inputs. No source change or lint suppression is
needed to retain these values.

MediaProfiles maps vocabulary only. DecoderCapabilities snapshots actual decoder
inventory, matches MIME, reads its actual capabilities and requires
isFormatSupported; absent/failed capability reads do not advertise a codec.
CodecReportReceiver likewise uses actual inventory/profile levels; newer hardware
classification calls are guarded at SDK29, and older ambiguous vendors remain
unknown. Numeric AV1 vocabulary alone never establishes AV1 availability.

The retained MediaProfilesTest, CodecReportTest, VideoCodecTest and
DecoderNegotiationTest pass offline, including SDK27/29/30/34 boundary cases,
unknown/absent capabilities, malformed selection/ranges and distinct AV1 versus
VP9 admission. Exact counts, source hashes, bytecode and logs accompany this
review. This resolves the advisory review while preserving explicit runtime
capability checks; no production function or accepted contract changes. Hosted
inventories are not native codec/device qualification, throughput or power proof.
No package transfer, hardware action or push occurred.
