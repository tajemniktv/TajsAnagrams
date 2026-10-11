use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn collect_source_files(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(root.join(directory))
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| directory.join(entry.unwrap().file_name()))
        .collect();
    entries.sort();
    for path in entries {
        let metadata = fs::symlink_metadata(root.join(&path))
            .unwrap_or_else(|error| panic!("cannot inspect {}: {error}", path.display()));
        assert!(
            !metadata.file_type().is_symlink(),
            "symlinked source paths are unsupported: {}",
            path.display()
        );
        if metadata.is_dir() {
            collect_source_files(root, &path, files);
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            files.push(path);
        }
    }
}

/// Stable content identity: includes dirty source changes, excludes checkout
/// location, timestamps, generated build products, and unrelated source files.
pub fn fingerprint(root: &Path) -> String {
    let mut files = vec![
        PathBuf::from("build.rs"),
        PathBuf::from("Cargo.toml"),
        PathBuf::from("../../Cargo.toml"),
        PathBuf::from("../../Cargo.lock"),
    ];
    collect_source_files(root, Path::new("src"), &mut files);
    files.sort();

    let mut digest = Sha256::new();
    for relative in files {
        // Keep logical names separate from filesystem paths: joining a parent
        // path to a Windows verbatim root normalizes away its `..` components.
        let name = relative.to_string_lossy().replace('\\', "/");
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        let path = root.join(relative);
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        digest.update((bytes.len() as u64).to_be_bytes());
        digest.update(bytes);
    }
    format!("core-sha256.{:x}", digest.finalize())
}

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=../../Cargo.toml");
    println!("cargo:rerun-if-changed=../../Cargo.lock");
    println!("cargo:rerun-if-changed=src");
    let identity = fingerprint(&root);
    println!("cargo:rustc-env=ANAGRAM_CORE_BUILD_ID={identity}");
}
