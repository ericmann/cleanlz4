//! The one function that compares cleanlz4 with the oracle (SPEC section 6.4).

use crate::block::{decompress_safe, Rejected};
use crate::config::Config;
use crate::frame::{decompress_frames, Decoded};
use cleanlz4::{
    decompress_block, decompress_frame_with, DecodeOptions, Dictionary, Error, FrameKind,
    FrameOutput,
};

/// Spare capacity given to the oracle's block decoder: it rejects some valid
/// blocks at exact capacity (docs/spikes/P1-06.md).
pub const BLOCK_SLACK: usize = 65_536;

/// The result of one comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Both sides reached the same verdict (possibly through a normalization).
    Agree { accepted: bool },
    /// The comparison is not meaningful, and why.
    Skipped(&'static str),
    /// A disagreement no normalization explains.
    Diverge(String),
}

/// A named, justified difference between the oracle and cleanlz4.
#[derive(Debug, Clone, Copy)]
pub struct Normalization {
    pub id: &'static str,
    pub what: &'static str,
    /// The document and section that permit or require cleanlz4's side.
    pub section: &'static str,
}

/// Every normalization, each with a DESIGN.md entry under "Differential rules".
pub const NORMALIZATIONS: [Normalization; 2] = [
    Normalization {
        id: "legacy-frame",
        what: "cleanlz4 accepts a legacy frame; the oracle's frame decoder does not know the legacy magic number",
        section: "Frame format, Legacy frame",
    },
    Normalization {
        id: "zero-offset",
        what: "cleanlz4 rejects a zero match offset; the oracle accepts it",
        section: "Block format, Compressed block format (offset)",
    },
];

/// cleanlz4's side of a comparison.
#[derive(Debug, Clone)]
pub struct Ours {
    pub result: Result<Vec<u8>, Error>,
    pub kinds: Vec<FrameKind>,
}

/// The id of the normalization that explains this disagreement, if any.
pub fn normalization_for(ours: &Ours, theirs: &Decoded) -> Option<&'static str> {
    match (&ours.result, theirs) {
        (Ok(_), Decoded::Rejected(_)) if ours.kinds.contains(&FrameKind::Legacy) => {
            Some("legacy-frame")
        }
        (Err(Error::ZeroOffset), Decoded::Accepted(_)) => Some("zero-offset"),
        _ => None,
    }
}

/// Compare two outcomes under output cap `cap`.
pub fn judge(ours: &Ours, theirs: &Decoded, cap: usize) -> Verdict {
    let ours_over = match &ours.result {
        Err(Error::OutputLimitExceeded) => true,
        Ok(v) => v.len() > cap,
        Err(_) => false,
    };
    if ours_over || matches!(theirs, Decoded::OverCap) {
        return Verdict::Skipped("output-cap");
    }
    match (&ours.result, theirs) {
        (Ok(a), Decoded::Accepted(b)) => {
            if a == b {
                Verdict::Agree { accepted: true }
            } else {
                Verdict::Diverge(format!(
                    "both accept but the outputs differ ({} and {} bytes)",
                    a.len(),
                    b.len()
                ))
            }
        }
        (Err(_), Decoded::Rejected(_)) => Verdict::Agree { accepted: false },
        (ours_result, _) => match normalization_for(ours, theirs) {
            Some(_) => Verdict::Agree {
                accepted: ours_result.is_ok(),
            },
            None => Verdict::Diverge(format!(
                "cleanlz4 {} but the oracle {}",
                match ours_result {
                    Ok(v) => format!("accepted ({} bytes)", v.len()),
                    Err(e) => format!("rejected ({e:?})"),
                },
                match theirs {
                    Decoded::Accepted(v) => format!("accepted ({} bytes)", v.len()),
                    Decoded::Rejected(why) => format!("rejected ({why})"),
                    Decoded::OverCap => String::from("went over the cap"),
                }
            )),
        },
    }
}

/// Run cleanlz4 on a frame flow through `run`, retrying with an empty
/// dictionary when it reports `DictionaryRequired`.
pub fn run_frames_with(
    input: &[u8],
    cfg: &Config,
    run: &dyn Fn(&[u8], &DecodeOptions) -> cleanlz4::Result<FrameOutput>,
) -> Ours {
    let mut options = DecodeOptions {
        max_output_bytes: cfg.diff.max_output_bytes,
        ..DecodeOptions::default()
    };
    let mut result = run(input, &options);
    if matches!(result, Err(Error::DictionaryRequired)) {
        options.dictionary = Some(Dictionary {
            bytes: Vec::new(),
            id: None,
        });
        result = run(input, &options);
    }
    match result {
        Ok(out) => Ours {
            result: Ok(out.data),
            kinds: out.kinds,
        },
        Err(e) => Ours {
            result: Err(e),
            kinds: Vec::new(),
        },
    }
}

/// Compare cleanlz4 with the oracle on a frame flow, with an injectable cleanlz4 side.
pub fn compare_frame_using(
    input: &[u8],
    cfg: &Config,
    run: &dyn Fn(&[u8], &DecodeOptions) -> cleanlz4::Result<FrameOutput>,
) -> (Verdict, Option<&'static str>) {
    let cap = cfg.diff.max_output_bytes as usize;
    let ours = run_frames_with(input, cfg, run);
    let theirs = decompress_frames(input, cap);
    let verdict = judge(&ours, &theirs, cap);
    let norm = match &verdict {
        Verdict::Agree { .. } => normalization_for(&ours, &theirs),
        _ => None,
    };
    (verdict, norm)
}

/// Compare cleanlz4 with the oracle on a frame flow.
pub fn compare_frame(input: &[u8], cfg: &Config) -> Verdict {
    compare_frame_using(input, cfg, &decompress_frame_with).0
}

/// Compare cleanlz4 with the oracle on one block, with `max_output` as the cap.
pub fn compare_block(input: &[u8], max_output: usize) -> Verdict {
    let ours = Ours {
        result: decompress_block(input, max_output),
        kinds: Vec::new(),
    };
    let theirs = match decompress_safe(input, max_output.saturating_add(BLOCK_SLACK)) {
        Ok(v) if v.len() > max_output => Decoded::OverCap,
        Ok(v) => Decoded::Accepted(v),
        Err(Rejected) => Decoded::Rejected(String::from("rejected")),
    };
    judge(&ours, &theirs, max_output)
}
