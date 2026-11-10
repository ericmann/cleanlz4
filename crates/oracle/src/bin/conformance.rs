//! `conformance --task <id> --categories <list|all> [--check]`: decode every
//! selected entry with cleanlz4 and write `testdata/results/conformance-<id>.json`.

use oracle::corpus::{conformance_dir, generate};
use oracle::manifest::{Entry, Manifest};
use oracle::results::{write_result, ConformanceResult, CATEGORIES};
use oracle::runner::{cleanlz4_decoder, run_entries};
use std::collections::BTreeSet;

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut task, mut categories, mut check) = (None, None, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--task" => {
                i += 1;
                task = args.get(i).cloned();
            }
            "--categories" => {
                i += 1;
                categories = args.get(i).cloned();
            }
            "--check" => check = true,
            other => return Err(format!("unknown argument {other}")),
        }
        i += 1;
    }
    let task = task.ok_or("missing --task")?;
    let spec = categories.ok_or("missing --categories")?;
    let selected: BTreeSet<String> = if spec == "all" {
        CATEGORIES.iter().map(|c| c.to_string()).collect()
    } else {
        spec.split(',').map(|c| c.trim().to_string()).collect()
    };

    let root = conformance_dir();
    let mut manifest = Manifest::load()?;
    let missing_suites: BTreeSet<String> = manifest
        .conformance
        .as_ref()
        .map(|c| {
            c.entries
                .iter()
                .filter(|e| selected.contains(&e.category) && !root.join(&e.file).is_file())
                .map(|e| e.suite.clone())
                .collect()
        })
        .unwrap_or_default();
    if manifest.conformance.is_none() || !missing_suites.is_empty() {
        let suites: Vec<String> = if missing_suites.is_empty() {
            oracle::corpus::SUITES
                .iter()
                .map(|s| s.to_string())
                .collect()
        } else {
            missing_suites.into_iter().collect()
        };
        generate(&suites)?;
        manifest = Manifest::load()?;
    }
    let conf = manifest.conformance.ok_or("no conformance section")?;
    let entries: Vec<&Entry> = conf
        .entries
        .iter()
        .filter(|e| selected.contains(&e.category))
        .collect();
    let report = run_entries(&entries, &root, &cleanlz4_decoder);
    for (id, why) in &report.failures {
        eprintln!("FAIL {id}: {why}");
    }
    println!(
        "conformance {task}: {} of {} passed",
        report.passed, report.total
    );
    if !check {
        let result = ConformanceResult {
            task: task.clone(),
            corpus_sha256: conf.corpus_sha256.clone(),
            total: report.total,
            passed: report.passed,
            failed: report.total - report.passed,
            by_category: report.by_category,
        };
        write_result(&format!("conformance-{task}.json"), &result)?;
    }
    Ok(report.failures.is_empty() && report.total > 0)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("conformance: {e}");
            std::process::exit(1);
        }
    }
}
