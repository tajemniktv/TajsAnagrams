mod benchmark;
mod contracts;
mod quality;
mod research;
mod stages;
mod verify;

use serde_json::Value;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_owned()
}
fn read_json(path: impl AsRef<Path>) -> Result<Value> {
    Ok(serde_json::from_reader(fs::File::open(path)?)?)
}
fn write_json(path: Option<&str>, value: &Value) -> Result<()> {
    let content = serde_json::to_string_pretty(value)? + "\n";
    if let Some(path) = path {
        let path = Path::new(path);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
    } else {
        print!("{content}");
    }
    Ok(())
}
fn digest(path: impl AsRef<Path>) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut f = fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let n = f.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        h.update(&bytes[..n]);
    }
    Ok(hex::encode(h.finalize()))
}
fn run(command: &mut Command) -> Result<()> {
    if !command.status()?.success() {
        return Err(format!("Command failed: {command:?}").into());
    }
    Ok(())
}

#[derive(Default)]
struct Args(BTreeMap<String, String>);
impl Args {
    fn parse(args: &[String], options: &[&str], switches: &[&str]) -> Result<Self> {
        let mut values = BTreeMap::new();
        let mut i = 0;
        while i < args.len() {
            let key = args[i].strip_prefix("--").ok_or("Expected --option")?;
            let value = if switches.contains(&key) {
                "true".into()
            } else if options.contains(&key) {
                i += 1;
                args.get(i)
                    .filter(|s| !s.starts_with("--"))
                    .ok_or("Missing option value")?
                    .clone()
            } else {
                return Err(format!("Unknown option --{key}").into());
            };
            if values.insert(key.into(), value).is_some() {
                return Err(format!("Duplicate --{key}").into());
            }
            i += 1;
        }
        Ok(Self(values))
    }
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
    fn value<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }
    fn required(&self, key: &str) -> Result<&str> {
        self.get(key)
            .ok_or_else(|| format!("Missing --{key}").into())
    }
    fn number(&self, key: &str, default: u64) -> Result<u64> {
        Ok(self
            .get(key)
            .map(str::parse)
            .transpose()?
            .unwrap_or(default))
    }
    fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
}

fn check() -> Result<()> {
    check_native_pipeline()?;
    run(Command::new("cargo").args(["fmt", "--all", "--", "--check"]))?;
    run(Command::new("cargo").args([
        "test",
        "--locked",
        "-p",
        "anagram-core",
        "-p",
        "anagram-cli",
        "-p",
        "xtask",
    ]))?;
    run(Command::new("cargo").args([
        "clippy",
        "--locked",
        "-p",
        "anagram-core",
        "-p",
        "anagram-cli",
        "-p",
        "xtask",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ]))?;
    contracts::generate(true)?;
    // AJV is an independently implemented schema validator. It does not execute Rust deserialization.
    #[cfg(windows)]
    let mut npm = {
        let mut c = Command::new("cmd");
        c.args(["/c", "npm"]);
        c
    };
    #[cfg(not(windows))]
    let mut npm = Command::new("npm");
    run(npm
        .args(["run", "check"])
        .current_dir(root().join("contracts")))
}

fn check_native_pipeline() -> Result<()> {
    fn scan(path: &Path) -> Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                scan(&entry.path())?;
            } else if entry.path().extension().is_some_and(|e| e == "py") {
                return Err(format!("Active Python source is retired; move disposable/history work under .codex: {}",entry.path().display()).into());
            }
        }
        Ok(())
    }
    for dir in ["tools", "tests"] {
        scan(&root().join(dir))?;
    }
    let invokes_python =
        regex::Regex::new(r"(?im)^\s*(?:-\s*run:\s*)?(?:python(?:3(?:\.\d+)?)?|pip(?:3)?)\s")?;
    for entry in fs::read_dir(root().join(".github/workflows"))? {
        let entry = entry?;
        let source = fs::read_to_string(entry.path())?;
        if source.contains("actions/setup-python") || invokes_python.is_match(&source) {
            return Err(format!(
                "Workflow requires retired Python runtime: {}",
                entry.path().display()
            )
            .into());
        }
    }
    println!("Active tools/tests/workflows are Python-free");
    Ok(())
}
fn main() {
    if let Err(error) = main_inner() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn main_inner() -> Result<()> {
    std::env::set_current_dir(root())?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") if args.len() == 1 => check(),
        Some("contracts")
            if args.len() == 1
                || args.get(1).map(String::as_str) == Some("--check") && args.len() == 2 =>
        {
            contracts::generate(args.len() == 2)
        }
        Some("quality") => quality::command(&args[1..]),
        Some("benchmark") => benchmark::command(&args[1..]),
        Some("research") => research::command(&args[1..]),
        None | Some("help" | "--help") => {
            println!(
                "cargo xtask check\ncargo xtask contracts [--check]\ncargo xtask quality --runs FILE [--cases FILE --ranking desktop|final_score|both --output FILE]\ncargo xtask benchmark [--cli PATH --before PATH --mode solve|generate --stages --output FILE]\ncargo xtask research lexicon --db FILE [--current FILE]\ncargo xtask research ngram --arpa FILE --groups FILE\ncargo xtask research tubelex --tubelex FILE [--unigrams FILE]\nUse --help after quality, benchmark or a research command for options."
            );
            Ok(())
        }
        _ => Err("Unknown command or unexpected arguments; use cargo xtask help".into()),
    }
}
