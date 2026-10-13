//! Explicit test child, not an implementation of the solver.
use std::{io::Read, time::Duration};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let mode = std::fs::read_to_string(&args[2]).unwrap();
    let mut stdin = String::new();
    std::io::stdin().read_to_string(&mut stdin).unwrap();
    match mode.trim() {
        "invalid" => print!("not json"),
        "array" => print!("[]"),
        "exit" => {
            print!("{{}}");
            std::process::exit(7);
        }
        "error" => {
            print!("{{\"error\":{{\"code\":\"timed_out\"}}}}");
            std::process::exit(1);
        }
        "sleep" => {
            std::thread::sleep(Duration::from_secs(5));
            print!("{{}}");
        }
        "large" => {
            for _ in 0..66 {
                print!("{}", "x".repeat(1024 * 1024));
            }
        }
        "success" => {
            std::thread::sleep(Duration::from_millis(80));
            print!("{{\"bags\":[[\"ab\"]],\"generated\":1,\"stop\":\"exhausted\"}}");
        }
        _ => panic!("Unknown process fixture"),
    }
}
