use super::*;
use std::os::unix::fs::symlink;

struct Fixture {
    _root: tempfile::TempDir,
    program: PathBuf,
    metadata: PathBuf,
    native: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("usr/bin");
        let libexec = root.path().join("usr/libexec");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::create_dir_all(&libexec).unwrap();
        let program = bin.join("ffmpeg");
        std::fs::write(&program, b"#!/bin/sh\n").unwrap();
        let native = libexec.join("ffmpeg");
        std::fs::write(&native, b"\x7fELFfixture").unwrap();
        let fixture = Self {
            metadata: bin.join("ffmpeg.blent-runtime.json"),
            program,
            native,
            _root: root,
        };
        fixture.write(serde_json::json!({"schema":1,"files":["libexec/ffmpeg"]}));
        fixture
    }
    fn write(&self, value: serde_json::Value) {
        std::fs::write(&self.metadata, serde_json::to_vec(&value).unwrap()).unwrap();
    }
}

#[test]
fn t1676_declared_components_are_rehashed_and_links_stay_inside_the_root() {
    let fixture = Fixture::new();
    let original = fingerprint(&fixture.program).unwrap();
    let alias = fixture.program.with_file_name("selected-ffmpeg");
    symlink(&fixture.program, &alias).unwrap();
    assert_eq!(fingerprint(&alias), Some(original));
    std::fs::write(&fixture.native, b"changed native encoder").unwrap();
    assert_ne!(fingerprint(&fixture.program).unwrap(), original);
    let library = fixture
        .native
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("lib/blent-ffmpeg");
    std::fs::create_dir_all(&library).unwrap();
    std::fs::write(library.join("codec.so"), b"codec one").unwrap();
    fixture.write(
        serde_json::json!({"schema":1,"files":["libexec/ffmpeg","lib/blent-ffmpeg/codec.so"]}),
    );
    let before = fingerprint(&fixture.program).unwrap();
    std::fs::write(library.join("codec.so"), b"codec two").unwrap();
    assert_ne!(fingerprint(&fixture.program).unwrap(), before);
    let outside = fixture._root.path().join("outside");
    std::fs::write(&outside, "external").unwrap();
    std::fs::remove_file(library.join("codec.so")).unwrap();
    symlink(outside, library.join("codec.so")).unwrap();
    assert!(fingerprint(&fixture.program).is_none());
}

#[test]
fn t1676_native_programs_work_without_metadata_but_opaque_scripts_do_not() {
    let fixture = Fixture::new();
    std::fs::remove_file(&fixture.metadata).unwrap();
    assert!(fingerprint(&fixture.program).is_none());
    assert!(fingerprint(&fixture.native).is_some());
    assert!(!native_elf(&fixture.program.with_file_name("missing")));
    assert!(fingerprint(&fixture.program.with_file_name("missing")).is_none());
    std::fs::write(&fixture.program, "").unwrap();
    assert!(!native_elf(&fixture.program));
}

#[test]
fn t1676_manifest_schema_and_count_bounds_reject_obsolete_or_unknown_inputs() {
    let fixture = Fixture::new();
    for value in [
        serde_json::json!({"schema":0,"files":["libexec/ffmpeg"]}),
        serde_json::json!({"schema":2,"files":["libexec/ffmpeg"]}),
        serde_json::json!({"schema":1,"files":[]}),
        serde_json::json!({"schema":1,"files":["other"]}),
        serde_json::json!({"schema":1,"files":["libexec/ffmpeg","libexec/ffmpeg"]}),
        serde_json::json!({"schema":1,"files":vec!["libexec/ffmpeg";33]}),
        serde_json::json!({"schema":1,"files":["libexec/ffmpeg"],"extra":true}),
        serde_json::json!({"files":["libexec/ffmpeg"]}),
    ] {
        fixture.write(value);
        assert!(fingerprint(&fixture.program).is_none());
    }
    std::fs::write(&fixture.metadata, vec![b' '; 16385]).unwrap();
    assert!(fingerprint(&fixture.program).is_none());
    std::fs::write(&fixture.metadata, b"{").unwrap();
    assert!(fingerprint(&fixture.program).is_none());
}

#[test]
fn t1676_paths_reject_escape_control_missing_and_nonregular_components() {
    let fixture = Fixture::new();
    for name in [
        "../outside",
        "/absolute",
        "lib//file",
        "./file",
        "lib/../file",
        "lib/./file",
        "lib/",
        "bad\0name",
        "bad\u{85}name",
        "missing",
        "libexec",
    ] {
        fixture.write(serde_json::json!({"schema":1,"files":["libexec/ffmpeg",name]}));
        assert!(fingerprint(&fixture.program).is_none(), "{name:?}");
    }
    fixture.write(serde_json::json!({"schema":1,"files":["libexec/ffmpeg","x".repeat(257)]}));
    assert!(fingerprint(&fixture.program).is_none());
    assert!(file_hash(&fixture.program.parent().unwrap()).is_none());
    std::fs::remove_file(&fixture.metadata).unwrap();
    symlink(&fixture.native, &fixture.metadata).unwrap();
    assert!(fingerprint(&fixture.program).is_none());
}

#[test]
fn t1676_file_size_and_manifest_exact_boundaries_are_enforced() {
    let fixture = Fixture::new();
    std::fs::File::create(&fixture.native)
        .unwrap()
        .set_len(536_870_912)
        .unwrap();
    assert!(file_hash(&fixture.native).is_some());
    std::fs::File::create(&fixture.native)
        .unwrap()
        .set_len(536_870_913)
        .unwrap();
    assert!(file_hash(&fixture.native).is_none());
    let mut content =
        serde_json::to_vec(&serde_json::json!({"schema":1,"files":["libexec/ffmpeg"]})).unwrap();
    content.resize(16_384, b' ');
    std::fs::write(&fixture.metadata, &content).unwrap();
    assert!(manifest(&fixture.metadata).is_some());
    content.push(b' ');
    std::fs::write(&fixture.metadata, content).unwrap();
    assert!(manifest(&fixture.metadata).is_none());
}

#[test]
fn t1676_hash_streaming_matches_sha256_across_chunk_boundaries() {
    let fixture = Fixture::new();
    for size in [0, 1, 65535, 65536, 65537] {
        let bytes = vec![0x5a; size];
        std::fs::write(&fixture.native, &bytes).unwrap();
        let expected: [u8; 32] = Sha256::digest(&bytes).into();
        assert_eq!(file_hash(&fixture.native), Some(expected));
    }
}

#[test]
fn t1676_maximum_component_count_and_path_length_are_admitted() {
    let fixture = Fixture::new();
    let root = fixture.program.parent().unwrap().parent().unwrap();
    let mut files = vec!["libexec/ffmpeg".to_owned()];
    for index in 0..31 {
        let name = format!("libexec/{index:0248}");
        assert_eq!(name.len(), 256);
        std::fs::write(root.join(&name), b"component").unwrap();
        files.push(name);
    }
    fixture.write(serde_json::json!({"schema":1,"files":files}));
    assert!(fingerprint(&fixture.program).is_some());
}
