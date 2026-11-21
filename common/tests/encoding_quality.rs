//! T1489: the configured VAAPI quality must reach an option the codec consumes.
use blent_config::encoding::Profile;

#[test]
fn t1489_each_vaapi_codec_receives_its_supported_quality_option() {
    for (encoder, wanted, forbidden) in [
        ("vp9_vaapi", "-global_quality", "-qp"),
        ("av1_vaapi", "-global_quality", "-qp"),
        ("h264_vaapi", "-qp", "-global_quality"),
        ("h264_vaapi_baseline", "-qp", "-global_quality"),
        ("hevc_vaapi", "-qp", "-global_quality"),
    ] {
        for quality in [blent_config::MIN_QUALITY, 18, blent_config::MAX_QUALITY] {
            let options = Profile::new(encoder, 30, 3000, quality)
                .unwrap()
                .cli_options(false);
            assert!(
                options.contains(&(wanted.into(), quality.to_string())),
                "{encoder}: {options:?}"
            );
            assert!(!options.iter().any(|(key, _)| key == forbidden));
            assert!(options.contains(&("-rc_mode".into(), "CQP".into())));
        }
    }
}
