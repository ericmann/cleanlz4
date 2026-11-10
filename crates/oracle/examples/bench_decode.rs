//! `bench_decode --task <id> [--repeats <n>] [--only <corpus name>]` writes
//! `testdata/results/bench-<id>.json`: decode throughput of cleanlz4 and the
//! oracle on every corpus file. Performance is reported, not a gate.

use oracle::bench::measure;
use oracle::config::Config;
use oracle::corpus::{corpus_file_inputs, make_frame, FrameCombo};
use oracle::manifest::Manifest;
use oracle::results::{write_result, BenchInput, BenchResult};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut task, mut repeats, mut only) = (None, None, None);
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--task" => {
                i += 1;
                task = args.get(i).cloned();
            }
            "--repeats" => {
                i += 1;
                repeats = args.get(i).and_then(|s| s.parse::<u64>().ok());
            }
            "--only" => {
                i += 1;
                // The Canterbury directory is named `cantrbry`; the manifest calls it `canterbury`.
                only = args.get(i).map(|o| {
                    if o == "cantrbry" {
                        String::from("canterbury")
                    } else {
                        o.clone()
                    }
                });
            }
            other => return Err(format!("unknown argument {other}")),
        }
        i += 1;
    }
    let task = task.ok_or("missing --task")?;
    let cfg = Config::load()?;
    let repeats = repeats.unwrap_or(cfg.bench.repeats);
    let manifest = Manifest::load()?;
    // 4 MiB blocks, independent, content checksum on, level 0.
    let combo = FrameCombo {
        block_size_id: 7,
        linked: false,
        block_checksum: false,
        content_checksum: true,
        content_size: false,
    };
    let mut inputs = Vec::new();
    for (name, path) in corpus_file_inputs(&manifest)? {
        if let Some(o) = &only {
            if !name.starts_with(&format!("{o}/")) {
                continue;
            }
        }
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let frame = make_frame(&data, &combo, 0);
        let rates = measure(&frame, data.len(), repeats);
        println!(
            "{name}: cleanlz4 {} MB/s, oracle {} MB/s",
            rates.cleanlz4_mb_s, rates.c_mb_s
        );
        inputs.push(BenchInput {
            name,
            bytes: data.len() as u64,
            cleanlz4_mb_s: rates.cleanlz4_mb_s,
            c_mb_s: rates.c_mb_s,
        });
    }
    if inputs.is_empty() {
        return Err("no inputs selected".into());
    }
    write_result(
        &format!("bench-{task}.json"),
        &BenchResult {
            task: task.clone(),
            inputs,
        },
    )?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("bench_decode: {e}");
        std::process::exit(1);
    }
}
