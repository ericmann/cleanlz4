//! `malformed --write` regenerates `testdata/malformed/`; `malformed --task <id>`
//! runs every file through cleanlz4 and writes `testdata/results/malformed-<id>.json`.

use oracle::config::Config;
use oracle::malformed::{index_cap, malformed_dir, run, write_all};

fn go() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut ok = true;
    let mut i = 0;
    let mut did = false;
    while i < args.len() {
        match args[i].as_str() {
            "--write" => {
                let cfg = Config::load()?;
                let n = write_all(&malformed_dir(), index_cap(&cfg))?;
                println!("malformed: wrote {n} files");
                did = true;
            }
            "--task" => {
                i += 1;
                let task = args.get(i).ok_or("missing task id")?;
                let r = run(task)?;
                println!(
                    "malformed {task}: {} of {} matched, {} panics",
                    r.matched_expected_error, r.total, r.panics
                );
                ok = r.matched_expected_error == r.total && r.panics == 0 && r.total > 0;
                did = true;
            }
            other => return Err(format!("unknown argument {other}")),
        }
        i += 1;
    }
    if !did {
        return Err("usage: malformed [--write] [--task <id>]".into());
    }
    Ok(ok)
}

fn main() {
    match go() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("malformed: {e}");
            std::process::exit(1);
        }
    }
}
