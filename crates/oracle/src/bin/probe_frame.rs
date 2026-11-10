//! Spike P2-09: how does the oracle's frame decoder treat each spec-permitted choice?

use oracle::craft_frame::probes;
use oracle::frame::{decompress_frames, Decoded};

const CAP: usize = 1 << 26;

fn verdict_oracle(b: &[u8]) -> String {
    match decompress_frames(b, CAP) {
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
    println!("| probe | oracle | cleanlz4 fixed | cleanlz4 actual |");
    println!("|---|---|---|---|");
    for p in probes() {
        let (_, actual) = verdict_ours(&p.bytes);
        println!(
            "| {} | {} | {} | {} |",
            p.id,
            verdict_oracle(&p.bytes),
            if p.fixed_accept { "accept" } else { "reject" },
            actual
        );
    }
}
