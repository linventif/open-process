//! Runs the Alpha algorithm on an event log and prints every step.
//!
//! ```sh
//! cargo run -p op-core --example alpha -- log.txt
//! echo "ABCD ACBD EF" | cargo run -p op-core --example alpha
//! ```

use op_core::mining::{EventLog, alpha};
use std::io::Read;

fn main() {
    let text = match std::env::args().nth(1) {
        Some(path) => std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {path}: {e}")),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).expect("stdin");
            s
        }
    };
    match EventLog::parse(&text) {
        Ok(log) => print!("{}", alpha(&log).report(&log)),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
