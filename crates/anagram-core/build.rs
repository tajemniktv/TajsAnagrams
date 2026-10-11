use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn collect_source_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_source_files(&path, files);
        } else if path.is_file() && path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// Stable content identity: includes dirty source changes, excludes checkout
/// location, timestamps, generated build products, and unrelated source files.
pub fn fingerprint(root: &Path) -> String {
    let source = root.join("src");
    let mut files = vec![
        root.join("build.rs"),
        root.join("Cargo.toml"),
        root.join("../../Cargo.toml"),
        root.join("../../Cargo.lock"),
    ];
    collect_source_files(&source, &mut files);
    files.sort();

    let mut digest = Sha256::new();
    for path in files {
        let relative = path
            .strip_prefix(root)
            .unwrap_or_else(|_| panic!("source path escaped package: {}", path.display()));
        let name = relative.to_string_lossy().replace('\\', "/");
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
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
