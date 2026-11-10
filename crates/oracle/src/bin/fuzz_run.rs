//! `fuzz_run --target <t> --task <id> [--smoke]`: run one fuzz target for the
//! configured time and write `testdata/results/fuzz-<t>-<id>.json`.

use oracle::config::{repo_root, Config};
use oracle::fuzzrun::{
    assemble_result, duration_seconds, file_names, fuzz_args, parse_final_stats, regression_name,
    regressions_dir, TARGETS,
};
use oracle::results::write_result;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Instant;

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut target, mut task, mut smoke) = (None, None, false);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                i += 1;
                target = args.get(i).cloned();
            }
            "--task" => {
                i += 1;
                task = args.get(i).cloned();
            }
            "--smoke" => smoke = true,
            other => return Err(format!("unknown argument {other}")),
        }
        i += 1;
    }
    let target = target.ok_or("missing --target")?;
    let task = task.unwrap_or_else(|| String::from("smoke"));
    if !TARGETS.contains(&target.as_str()) {
        return Err(format!("unknown target {target}"));
    }
    let cfg = Config::load()?;
    let root = repo_root();
    let corpus = root.join("fuzz").join("corpus").join(&target);
    let artifacts = root.join("fuzz").join("artifacts").join(&target);
    let logs = root.join("fuzz").join("logs");
    let divergences = if smoke {
        std::env::temp_dir().join(format!(
            "cleanlz4-smoke-div-{}-{target}",
            std::process::id()
        ))
    } else {
        root.join("testdata").join("results").join("divergences")
    };
    for d in [&corpus, &logs, &divergences] {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    // Seed inputs: the malformed corpus, when present.
    let seeds = root.join("testdata").join("malformed");
    if let Ok(rd) = std::fs::read_dir(&seeds) {
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            if p.is_file() && p.extension().is_some_and(|x| x != "json") {
                let _ = std::fs::copy(&p, corpus.join(e.file_name()));
            }
        }
    }
    let artifacts_before = file_names(&artifacts);
    let divergences_before = file_names(&divergences);

    let mut child = Command::new("cargo")
        .args(fuzz_args(&cfg, &target, smoke))
        .current_dir(&root)
        .env("CLEANLZ4_DIVERGENCE_DIR", &divergences)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start cargo fuzz: {e}"))?;
    let started = Instant::now();
    let (tx, rx) = mpsc::channel::<String>();
    let mut readers = Vec::new();
    if let Some(out) = child.stdout.take() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        }));
    }
    if let Some(err) = child.stderr.take() {
        let tx = tx.clone();
        readers.push(std::thread::spawn(move || {
            for line in BufReader::new(err).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        }));
    }
    drop(tx);
    let log_path = logs.join(format!("{target}-{task}.log"));
    let mut log =
        std::fs::File::create(&log_path).map_err(|e| format!("{}: {e}", log_path.display()))?;
    let mut all = String::new();
    for line in rx {
        let _ = writeln!(log, "[{:>6}s] {line}", started.elapsed().as_secs());
        all.push_str(&line);
        all.push('\n');
    }
    for r in readers {
        let _ = r.join();
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let seconds = started.elapsed().as_secs();

    let new_artifacts: Vec<String> = file_names(&artifacts)
        .difference(&artifacts_before)
        .cloned()
        .collect();
    let new_divergences = file_names(&divergences)
        .difference(&divergences_before)
        .filter(|n| n.ends_with(".bin"))
        .count() as u64;
    // Keep every crash as a regression input.
    let reg = regressions_dir();
    if !new_artifacts.is_empty() {
        std::fs::create_dir_all(&reg).map_err(|e| e.to_string())?;
    }
    for name in &new_artifacts {
        let bytes = std::fs::read(artifacts.join(name)).map_err(|e| e.to_string())?;
        std::fs::write(reg.join(regression_name(&target, &bytes)), &bytes)
            .map_err(|e| e.to_string())?;
    }
    let crashes = new_artifacts.len() as u64;
    let runs = parse_final_stats(&all);
    if !status.success() && crashes == 0 {
        return Err(format!(
            "the fuzzer stopped early without a crash ({status}); see {}",
            log_path.display()
        ));
    }
    let runs = match runs {
        Some(r) => r,
        None if crashes > 0 => 0,
        None => {
            return Err(format!(
                "no final statistics in {} (the fuzzer did not run)",
                log_path.display()
            ))
        }
    };
    let result = assemble_result(
        &cfg,
        &task,
        &target,
        seconds,
        runs,
        crashes,
        new_divergences,
        file_names(&corpus).len() as u64,
    );
    let name = format!("fuzz-{target}-{task}.json");
    let path = if smoke {
        let dir = std::env::temp_dir().join(format!("cleanlz4-smoke-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut text = serde_json::to_string_pretty(&result).map_err(|e| e.to_string())?;
        text.push('\n');
        let p = dir.join(&name);
        std::fs::write(&p, text).map_err(|e| e.to_string())?;
        p
    } else {
        write_result(&name, &result)?
    };
    println!(
        "fuzz_run {target} {task}: {runs} runs in {seconds}s (limit {}), crashes {crashes}, divergences {new_divergences}; {}",
        duration_seconds(&cfg, smoke),
        path.display()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("fuzz_run: {e}");
        std::process::exit(1);
    }
}
