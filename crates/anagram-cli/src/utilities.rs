//! CLI syntax and stdin ownership; reusable operations live in anagram-data.
use anagram_core::{control::Control, policy::DeploymentLimits, request::Error};
use serde_json::Value;
use std::{
    ffi::OsString,
    io::{self, Read},
};
fn err(e: impl std::fmt::Display) -> Error {
    Error::new("tool_error", e.to_string())
}
pub fn is_command(name: &str) -> bool {
    matches!(
        name,
        "ranker-build" | "ranker-train" | "ranker-rank" | "prepare-dictionary" | "build-phrases"
    )
}
pub fn run(args: &[OsString], control: &Control) -> Result<Value, Error> {
    run_with_limits(args, control, &DeploymentLimits::default())
}

pub fn run_with_limits(
    args: &[OsString],
    control: &Control,
    limits: &DeploymentLimits,
) -> Result<Value, Error> {
    if args
        .first()
        .is_some_and(|arg| arg.to_string_lossy().starts_with("ranker-"))
    {
        let bytes = read_stdin(control, 16 * 1024 * 1024)?;
        run_with_input_and_limits(args, control, &mut io::Cursor::new(bytes), limits)
    } else {
        run_with_input_and_limits(args, control, &mut io::empty(), limits)
    }
}

/// CLI-only input owner: a deadline must also work while a pipe writer stays open.
/// On timeout the CLI exits; it never joins a thread blocked in an OS stdin read.
pub fn read_stdin(control: &Control, limit: usize) -> Result<Vec<u8>, Error> {
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = io::stdin()
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    loop {
        control
            .check()
            .map_err(|reason| Error::new(reason, reason))?;
        match receive.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(result) => {
                let bytes = result.map_err(err)?;
                if bytes.len() > limit {
                    return Err(Error::new("request_too_large", "Input exceeds byte limit"));
                }
                return Ok(bytes);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(err(error)),
        }
    }
}
pub fn run_with_input(
    args: &[OsString],
    control: &Control,
    source: &mut dyn Read,
) -> Result<Value, Error> {
    run_with_input_and_limits(args, control, source, &DeploymentLimits::default())
}

pub fn run_with_input_and_limits(
    args: &[OsString],
    control: &Control,
    source: &mut dyn Read,
    limits: &DeploymentLimits,
) -> Result<Value, Error> {
    use anagram_data::Operation;
    let operation = match args.first().and_then(|a| a.to_str()).unwrap_or("") {
        "ranker-build" if (2..=3).contains(&args.len()) => Operation::BuildGroups {
            wordnet: args[1].clone().into(),
            phrase: args.get(2).cloned().map(Into::into),
        },
        "ranker-train" if args.len() == 2 => Operation::Train {
            output: args[1].clone().into(),
        },
        "ranker-rank" if args.len() == 2 => Operation::Rank {
            model: args[1].clone().into(),
        },
        "prepare-dictionary" if args.len() == 4 => Operation::Dictionary {
            base: args[1].clone().into(),
            unigrams: args[2].clone().into(),
            output: args[3].clone().into(),
        },
        "build-phrases" if args.len() >= 3 => Operation::Phrases {
            output: args[1].clone().into(),
            sources: args[2..].iter().cloned().map(Into::into).collect(),
        },
        _ => {
            return Err(err(
                "Usage: ranker-build WORDNET [PHRASE_DB] < cases.json | ranker-train NEW_MODEL < groups.json | ranker-rank MODEL < items.json | prepare-dictionary BASE UNIGRAMS NEW_DICTIONARY | build-phrases NEW_DB TITLE_FILE[.gz] ...",
            ));
        }
    };
    anagram_data::execute(operation, control, source, limits)
}
