//! The `lz4` command-line program, run as a black box (SPEC section 6.5, last
//! paragraph). Used only to make legacy frames.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("cleanlz4-cli-{}-{n}-{tag}", std::process::id()))
}

/// The first line `lz4 -V` prints.
pub fn cli_version() -> String {
    let out = match Command::new("lz4").arg("-V").output() {
        Ok(o) => o,
        Err(e) => panic!("cannot run lz4: {e}"),
    };
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text.lines().next().unwrap_or("").trim().to_string()
}

/// Compress `data` into a legacy frame with `lz4 -l`, through temporary files.
pub fn compress_legacy(data: &[u8]) -> Vec<u8> {
    let input = scratch_path("in");
    let output = scratch_path("out");
    if let Err(e) = std::fs::write(&input, data) {
        panic!("cannot write {}: {e}", input.display());
    }
    let status = Command::new("lz4")
        .args(["-l", "-f", "-q"])
        .arg(&input)
        .arg(&output)
        .status();
    let ok = matches!(&status, Ok(s) if s.success());
    let result = std::fs::read(&output);
    let _ = std::fs::remove_file(&input);
    let _ = std::fs::remove_file(&output);
    match (ok, result) {
        (true, Ok(bytes)) => bytes,
        (_, _) => panic!("lz4 -l failed: {status:?}"),
    }
}
