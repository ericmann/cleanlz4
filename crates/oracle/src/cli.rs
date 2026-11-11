//! The `lz4` command-line program, run as a black box (SPEC section 6.5, last
//! paragraph). It makes legacy frames (`lz4 -l`) and is the oracle for
//! dictionary decoding (`lz4 -d -D <dict>`), SPEC section 6.5.

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

/// Wrap one block in a minimal frame: independent blocks, 4 MiB maximum, no
/// checksums, no content size, one data block, end mark (Frame format, "Frame
/// Descriptor", "Data Blocks"). `block` is stored as a compressed block.
pub fn wrap_block_in_frame(block: &[u8]) -> Vec<u8> {
    let flg: u8 = 0x60;
    let bd: u8 = 0x70;
    let hc = ((cleanlz4::xxh32::xxh32(&[flg, bd], 0) >> 8) & 0xff) as u8;
    let mut out = vec![0x04, 0x22, 0x4D, 0x18, flg, bd, hc];
    let size = u32::try_from(block.len()).unwrap_or(0);
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(block);
    out.extend_from_slice(&[0, 0, 0, 0]);
    out
}

/// Decode `frame` with `lz4 -d -D <dict> -c`, through temporary files. Exit
/// status 0 is `Ok(stdout)`; anything else is `Rejected`.
pub fn decompress_frame_with_dict(
    frame: &[u8],
    dict: &[u8],
) -> Result<Vec<u8>, crate::block::Rejected> {
    let frame_path = scratch_path("frame");
    let dict_path = scratch_path("dict");
    if std::fs::write(&frame_path, frame).is_err() || std::fs::write(&dict_path, dict).is_err() {
        panic!("cannot write temporary files for lz4");
    }
    let out = Command::new("lz4")
        .args(["-d", "-q", "-c", "-D"])
        .arg(&dict_path)
        .arg(&frame_path)
        .output();
    let _ = std::fs::remove_file(&frame_path);
    let _ = std::fs::remove_file(&dict_path);
    match out {
        Ok(o) if o.status.success() => Ok(o.stdout),
        Ok(_) => Err(crate::block::Rejected),
        Err(e) => panic!("cannot run lz4: {e}"),
    }
}

/// Decode a bare block that depends on `dict`, through the `lz4` program.
pub fn decompress_block_with_dict(
    block: &[u8],
    dict: &[u8],
) -> Result<Vec<u8>, crate::block::Rejected> {
    decompress_frame_with_dict(&wrap_block_in_frame(block), dict)
}
