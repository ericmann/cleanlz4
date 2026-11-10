//! Chunking invariance: streaming with any input and output chunk sizes gives
//! exactly the one-shot result (SPEC section 8, Phase 3).

use cleanlz4::{decompress_frame, DecodeOptions, Decoder, Error, Status};
use oracle::block::compress_default;
use oracle::config::Config;
use oracle::craft_frame::{legacy, skippable};
use oracle::frame::{compress_frame, Prefs};
use oracle::prng::Prng;
use oracle::synth::word_salad;

/// Every case's random choices, drawn first so a guard test can count them.
struct Plan {
    seed: u64,
    len: usize,
    in_chunk: usize,
    out_chunk: usize,
    /// Which of the seven regimes each chunk size came from.
    in_regime: usize,
    out_regime: usize,
    rng: Prng,
}

fn case_seed(cfg: &Config, case: u64) -> u64 {
    cfg.fuzz.seed ^ case.wrapping_mul(0x9E37_79B9_7F4A_7C15)
}

/// A chunk size and the index of the regime it came from.
fn regime(rng: &mut Prng, len: usize) -> (usize, usize) {
    let mut pick = rng.below(7);
    // The one-byte regimes are only for inputs that stay quick.
    if len > 4096 && pick == 0 {
        pick = 1;
    }
    let size = match pick {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 7,
        4 => 64,
        5 => 4096,
        _ => 1 + rng.below(70_000) as usize,
    };
    (size, pick as usize)
}

fn plan(cfg: &Config, case: u64) -> Plan {
    let seed = case_seed(cfg, case);
    let mut rng = Prng::new(seed);
    let big = case % 50 == 49;
    let len = rng.below(if big { 300 << 10 } else { 16 << 10 }) as usize;
    let (in_chunk, in_regime) = regime(&mut rng, len);
    let (out_chunk, out_regime) = regime(&mut rng, len);
    Plan {
        seed,
        len,
        in_chunk,
        out_chunk,
        in_regime,
        out_regime,
        rng,
    }
}

/// Mixed compressibility: random, word salad, runs, or a blend.
fn make_input(rng: &mut Prng, len: usize) -> Vec<u8> {
    match rng.below(4) {
        0 => {
            let mut v = vec![0u8; len];
            rng.fill(&mut v);
            v
        }
        1 => word_salad(rng.next_u64(), len),
        2 => {
            let mut v = Vec::with_capacity(len);
            while v.len() < len {
                let run = 1 + rng.below(300) as usize;
                let b = rng.next_u64() as u8;
                v.extend(std::iter::repeat_n(b, run.min(len - v.len())));
            }
            v
        }
        _ => {
            let mut v = Vec::with_capacity(len);
            while v.len() < len {
                let part = (1 + rng.below(2000) as usize).min(len - v.len());
                if rng.below(2) == 0 {
                    let mut r = vec![0u8; part];
                    rng.fill(&mut r);
                    v.extend(r);
                } else {
                    v.extend(word_salad(rng.next_u64(), part));
                }
            }
            v
        }
    }
}

/// A random frame of `data` and whether its blocks are linked.
fn make_frame(cfg: &Config, rng: &mut Prng, data: &[u8]) -> (Vec<u8>, bool) {
    let levels = &cfg.conformance.frame_levels;
    let prefs = Prefs {
        block_size_id: 4 + rng.below(4) as u8,
        linked: rng.below(2) == 0,
        block_checksum: rng.below(2) == 0,
        content_checksum: rng.below(2) == 0,
        content_size: (rng.below(2) == 0).then_some(data.len() as u64),
        level: levels[rng.below(levels.len() as u64) as usize],
        ..Prefs::default()
    };
    (compress_frame(&[data], &prefs), prefs.linked)
}

/// What a case decodes and the shape it exercises.
struct Built {
    input: Vec<u8>,
    expected: Vec<u8>,
    linked: bool,
    /// 0 single frame, 1 two frames, 2 skippable first, 3 legacy.
    mix: usize,
}

/// The input to decode and the bytes it must decode to.
fn build(cfg: &Config, case: u64, p: &mut Plan) -> Built {
    let data = make_input(&mut p.rng, p.len);
    let (frame, linked) = make_frame(cfg, &mut p.rng, &data);
    let done = |input: Vec<u8>, expected: Vec<u8>, mix: usize| Built {
        input,
        expected,
        linked,
        mix,
    };
    if case % 10 != 9 {
        return done(frame, data, 0);
    }
    match (case / 10) % 3 {
        0 => {
            let len2 = p.rng.below(4096) as usize;
            let other = make_input(&mut p.rng, len2);
            let (second, _) = make_frame(cfg, &mut p.rng, &other);
            done([frame, second].concat(), [data, other].concat(), 1)
        }
        1 => {
            let user = p.rng.below(100) as usize;
            let mut bytes = vec![0u8; user];
            p.rng.fill(&mut bytes);
            let low = p.rng.below(16) as u8;
            done([skippable(low, &bytes), frame].concat(), data, 2)
        }
        _ => {
            let block = compress_default(&data);
            let zero_end = p.rng.below(2) == 0;
            done(legacy(&[block], zero_end), data, 3)
        }
    }
}

/// Stream `input` in `in_chunk`-byte pieces into `out_chunk`-byte buffers.
fn stream(input: &[u8], in_chunk: usize, out_chunk: usize) -> Result<(Vec<u8>, usize), Error> {
    let mut d = Decoder::new(DecodeOptions::default());
    let mut buf = vec![0u8; out_chunk];
    let mut out = Vec::new();
    let mut pos = 0usize;
    let bound = 100 * (input.len() + 100) + out_chunk;
    for _ in 0..bound {
        let end = pos.saturating_add(in_chunk).min(input.len());
        let p = d.decode(&input[pos..end], &mut buf, end == input.len())?;
        pos += p.consumed;
        out.extend_from_slice(&buf[..p.produced]);
        if p.status == Status::Done {
            return Ok((out, pos));
        }
    }
    panic!("the decoder made no progress");
}

/// Run one case: streamed equals one-shot equals the original. Returns its shape.
fn check_case(cfg: &Config, case: u64) -> (usize, usize, bool, usize) {
    let mut p = plan(cfg, case);
    let built = build(cfg, case, &mut p);
    let tag = format!(
        "case {case} seed {} len {} in {} out {}",
        p.seed, p.len, p.in_chunk, p.out_chunk
    );
    let one_shot = decompress_frame(&built.input).unwrap_or_else(|e| panic!("{tag}: one-shot {e}"));
    assert_eq!(one_shot, built.expected, "{tag}: one-shot");
    let (out, consumed) =
        stream(&built.input, p.in_chunk, p.out_chunk).unwrap_or_else(|e| panic!("{tag}: {e}"));
    assert_eq!(out, one_shot, "{tag}: streamed");
    assert_eq!(consumed, built.input.len(), "{tag}: consumed");
    (p.in_regime, p.out_regime, built.linked, built.mix)
}

#[test]
fn chunking_invariance_random_chunks() {
    let cfg = Config::load().expect("config");
    for case in 0..cfg.stream.chunking_cases {
        check_case(&cfg, case);
    }
}

/// Prints the wall time and how many of the shapes (input regime, output
/// regime, linked or independent, frame kind mix) a run hits. The case count is
/// `CHUNKING_CASES_OVERRIDE`, else the config key.
#[test]
#[ignore]
fn chunking_coverage_report() {
    let cfg = Config::load().expect("config");
    let cases = std::env::var("CHUNKING_CASES_OVERRIDE")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(cfg.stream.chunking_cases);
    let start = std::time::Instant::now();
    let mut shapes = std::collections::BTreeSet::new();
    for case in 0..cases {
        shapes.insert(check_case(&cfg, case));
    }
    // 7 regimes in, 7 out, 2 linkage settings, 4 kind mixes: every shape is
    // reachable, since the one-byte regimes only need an input up to 4 KiB.
    let reachable = 7 * 7 * 2 * 4;
    println!(
        "chunking coverage: cases {cases}, wall {:.2?}, shapes {} of {reachable}",
        start.elapsed(),
        shapes.len()
    );
}

#[test]
fn chunking_invariance_on_errors() {
    let cfg = Config::load().expect("config");
    for case in 0..cfg.stream.chunking_cases / 10 {
        let mut p = plan(&cfg, case);
        let mut input = build(&cfg, case, &mut p).input;
        if p.rng.below(2) == 0 {
            let cut = p.rng.below(input.len() as u64) as usize;
            input.truncate(cut);
        } else {
            let at = p.rng.below(input.len() as u64) as usize;
            input[at] ^= 1 + p.rng.below(255) as u8;
        }
        let tag = format!(
            "case {case} seed {} len {} in {} out {}",
            p.seed, p.len, p.in_chunk, p.out_chunk
        );
        let one_shot: Result<Vec<u8>, Error> = decompress_frame(&input);
        let streamed = stream(&input, p.in_chunk, p.out_chunk).map(|(out, _)| out);
        assert_eq!(streamed, one_shot, "{tag}");
    }
}

#[test]
fn regimes_include_one_byte_input_and_one_byte_output() {
    let cfg = Config::load().expect("config");
    let (mut one_in, mut one_out) = (false, false);
    for case in 0..cfg.stream.chunking_cases {
        let p = plan(&cfg, case);
        one_in |= p.in_chunk == 1;
        one_out |= p.out_chunk == 1;
    }
    assert!(one_in, "no case drew one-byte input chunks");
    assert!(one_out, "no case drew one-byte output buffers");
}
