//! `gen_corpus --suites <list|all>` regenerates conformance files and the
//! manifest's conformance section.

use oracle::corpus::{generate, SUITES};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut suites: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--suites" {
            i += 1;
            suites = args.get(i).cloned();
        }
        i += 1;
    }
    let spec = suites.ok_or("usage: gen_corpus --suites <list|all>")?;
    let list: Vec<String> = if spec == "all" {
        SUITES.iter().map(|s| s.to_string()).collect()
    } else {
        spec.split(',').map(|s| s.trim().to_string()).collect()
    };
    generate(&list)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("gen_corpus: {e}");
        std::process::exit(1);
    }
}
