#[allow(dead_code)]
#[path = "../build.rs"]
mod build_script;
use std::{
    fs,
    path::{Path, PathBuf},
};

struct Scratch(PathBuf);
impl Scratch {
    fn new(name: &str) -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.codex/temp");
        fs::create_dir_all(&root).unwrap();
        let path = root
            .canonicalize()
            .unwrap()
            .join(format!("build-id-{}-{name}", std::process::id()));
        fs::create_dir(&path).unwrap();
        let package = path.join("crates/core");
        fs::create_dir_all(package.join("src/nested")).unwrap();
        for (name, contents) in [
            ("Cargo.toml", "workspace"),
            ("Cargo.lock", "lock"),
            ("crates/core/Cargo.toml", "package"),
            ("crates/core/build.rs", "build"),
            ("crates/core/src/lib.rs", "mod solver;"),
            ("crates/core/src/nested/solver.rs", "fn solve() {}"),
        ] {
            fs::write(path.join(name), contents).unwrap();
        }
        Self(path)
    }
    fn package(&self) -> PathBuf {
        self.0.join("crates/core")
    }
}

#[cfg(unix)]
#[test]
fn source_symlinks_fail_closed_without_recursing_or_ignoring_code() {
    use std::os::unix::fs::symlink;
    let scratch = Scratch::new("symlinks");
    let link = scratch.package().join("src/loop");
    symlink(scratch.package().join("src"), &link).unwrap();
    assert!(std::panic::catch_unwind(|| build_script::fingerprint(&scratch.package())).is_err());
    fs::remove_file(link).unwrap();
    let outside = scratch.0.join("outside.rs");
    fs::write(&outside, "fn external() {}").unwrap();
    symlink(outside, scratch.package().join("src/external.rs")).unwrap();
    assert!(std::panic::catch_unwind(|| build_script::fingerprint(&scratch.package())).is_err());
}
impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn normalized_package_without_workspace_files_has_a_content_identity() {
    let scratch = Scratch::new("packaged");
    let workspace_id = build_script::fingerprint(&scratch.package());
    fs::remove_file(scratch.0.join("Cargo.toml")).unwrap();
    fs::remove_file(scratch.0.join("Cargo.lock")).unwrap();
    let packaged_id = build_script::fingerprint(&scratch.package());
    assert_ne!(workspace_id, packaged_id);
    assert_eq!(packaged_id, build_script::fingerprint(&scratch.package()));
    fs::write(
        scratch.package().join("Cargo.lock"),
        "normalized package lock",
    )
    .unwrap();
    assert_ne!(packaged_id, build_script::fingerprint(&scratch.package()));
}

#[test]
fn source_fingerprint_matches_compiled_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        build_script::fingerprint(root),
        env!("ANAGRAM_CORE_BUILD_ID")
    );
    // Windows canonical paths use a verbatim prefix and normalize parent joins.
    assert_eq!(
        build_script::fingerprint(&root.canonicalize().unwrap()),
        env!("ANAGRAM_CORE_BUILD_ID")
    );
}

#[test]
fn fingerprint_is_reproducible_across_paths_mtimes_and_unrelated_files() {
    let first = Scratch::new("first");
    let second = Scratch::new("second");
    let expected = build_script::fingerprint(&first.package());
    fs::write(second.package().join("src/lib.rs"), "mod solver;").unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(second.package().join("src/lib.rs"))
        .unwrap()
        .set_times(
            std::fs::FileTimes::new().set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1),
            ),
        )
        .unwrap();
    fs::write(second.package().join("src/notes.md"), "unrelated").unwrap();
    fs::write(second.0.join("unrelated.txt"), "unrelated").unwrap();
    assert_eq!(expected, build_script::fingerprint(&second.package()));
    assert_eq!(expected, build_script::fingerprint(&second.package()));
}

#[test]
fn dirty_semantic_sources_manifests_lock_and_added_deleted_modules_change_identity() {
    let scratch = Scratch::new("dirty");
    let expected = build_script::fingerprint(&scratch.package());
    for name in [
        "Cargo.toml",
        "Cargo.lock",
        "crates/core/Cargo.toml",
        "crates/core/build.rs",
        "crates/core/src/lib.rs",
        "crates/core/src/nested/solver.rs",
    ] {
        let path = scratch.0.join(name);
        let original = fs::read(&path).unwrap();
        let mut changed = original.clone();
        changed.extend_from_slice(b"changed");
        fs::write(&path, changed).unwrap();
        assert_ne!(
            expected,
            build_script::fingerprint(&scratch.package()),
            "{name}"
        );
        fs::write(&path, original).unwrap();
        assert_eq!(expected, build_script::fingerprint(&scratch.package()));
    }
    let added = scratch.package().join("src/nested/extra.rs");
    fs::write(&added, "fn extra() {}").unwrap();
    assert_ne!(expected, build_script::fingerprint(&scratch.package()));
    fs::remove_file(&added).unwrap();
    assert_eq!(expected, build_script::fingerprint(&scratch.package()));
}
