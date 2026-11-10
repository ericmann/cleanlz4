//! `manifest corpora --fetched-at <time>` writes `testdata/MANIFEST.json`.

use oracle::manifest::{build_corpora, collect_tools, Manifest};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("corpora") {
        return Err("usage: manifest corpora --fetched-at <UTC time>".into());
    }
    let mut fetched_at = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--fetched-at" {
            i += 1;
            fetched_at = args.get(i).cloned();
        }
        i += 1;
    }
    let fetched_at = fetched_at.ok_or("missing --fetched-at")?;
    let manifest = Manifest {
        version: 1,
        tools: collect_tools()?,
        corpora: build_corpora(&fetched_at)?,
        conformance: None,
    };
    manifest.save()
}

fn main() {
    if let Err(e) = run() {
        eprintln!("manifest: {e}");
        std::process::exit(1);
    }
}
