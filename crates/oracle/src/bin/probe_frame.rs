//! Spike P2-09: how does the oracle's frame decoder treat each spec-permitted choice?

use oracle::config::Config;
use oracle::craft_frame::probes;
use oracle::frame::{decompress_frames, Decoded};

fn verdict_oracle(b: &[u8], cap: usize) -> String {
    match decompress_frames(b, cap) {
        Decoded::Accepted(v) => format!("accept ({} bytes)", v.len()),
        Decoded::Rejected(why) => format!("reject ({why})"),
        Decoded::OverCap => "over cap".to_string(),
    }
}

fn verdict_ours(b: &[u8]) -> (bool, String) {
    match cleanlz4::decompress_frame(b) {
        Ok(v) => (true, format!("accept ({} bytes)", v.len())),
        Err(e) => (false, format!("reject ({e:?})")),
    }
}

fn main() {
    let cap = Config::load()
        .ok()
        .and_then(|c| usize::try_from(c.diff.max_output_bytes).ok())
        .expect("config diff.max_output_bytes");
    println!("| probe | oracle | cleanlz4 fixed | cleanlz4 actual |");
    println!("|---|---|---|---|");
    for p in probes() {
        let (_, actual) = verdict_ours(&p.bytes);
        println!(
            "| {} | {} | {} | {} |",
            p.id,
            verdict_oracle(&p.bytes, cap),
            if p.fixed_accept { "accept" } else { "reject" },
            actual
        );
    }
}
