use cleanlz4::xxh32::xxh32;
use cleanlz4::{DecodeOptions, Dictionary, Error, FrameKind, FrameOutput};
use oracle::config::Config;
use oracle::craft::{build_block, build_block_without_final, expected_output, Seq};
use oracle::craft_frame::probes;
use oracle::diff::{
    compare_block, compare_frame, compare_frame_using, judge, normalization_for, Ours, Verdict,
    NORMALIZATIONS,
};
use oracle::frame::{compress_frame, Decoded, Prefs};
use oracle::synth::{inputs, word_salad};
use std::cell::Cell;

#[test]
fn valid_oracle_frames_compare_as_agree() {
    let cfg = Config::load().expect("config");
    for (name, data) in inputs(cfg.fuzz.seed).into_iter().take(14) {
        for linked in [false, true] {
            let prefs = Prefs {
                block_size_id: 4,
                linked,
                content_checksum: true,
                ..Prefs::default()
            };
            let f = compress_frame(&[&data], &prefs);
            assert_eq!(
                compare_frame(&f, &cfg),
                Verdict::Agree { accepted: true },
                "{name}"
            );
        }
    }
}

#[test]
fn every_probe_is_agree_or_a_named_normalization() {
    let cfg = Config::load().expect("config");
    for p in probes() {
        let (verdict, norm) = compare_frame_using(&p.bytes, &cfg, &cleanlz4::decompress_frame_with);
        assert!(
            matches!(verdict, Verdict::Agree { .. }),
            "{}: {verdict:?}",
            p.id
        );
        if let Some(id) = norm {
            assert!(
                NORMALIZATIONS.iter().any(|n| n.id == id),
                "{}: unlisted normalization {id}",
                p.id
            );
        }
        if p.id == "Dict-ID and no dictionary" {
            // The comparison retries with an empty dictionary, so its verdict is
            // "accepted" by design; the fixed verdict is cleanlz4's own.
            assert_eq!(
                cleanlz4::decompress_frame_with(&p.bytes, &cleanlz4::DecodeOptions::default())
                    .map(|o| o.data),
                Err(Error::DictionaryRequired),
                "{}",
                p.id
            );
            continue;
        }
        let accepted = matches!(verdict, Verdict::Agree { accepted: true });
        assert_eq!(accepted, p.fixed_accept, "{}", p.id);
    }
}

fn ours(result: Result<Vec<u8>, Error>) -> Ours {
    Ours {
        result,
        kinds: vec![],
    }
}

#[test]
fn unequal_outputs_are_reported_as_diverge() {
    let v = judge(
        &ours(Ok(b"abc".to_vec())),
        &Decoded::Accepted(b"abd".to_vec()),
        100,
    );
    assert!(matches!(v, Verdict::Diverge(_)), "{v:?}");
    let v = judge(
        &ours(Ok(b"abc".to_vec())),
        &Decoded::Accepted(b"abc".to_vec()),
        100,
    );
    assert_eq!(v, Verdict::Agree { accepted: true });
}

#[test]
fn dictionary_required_is_retried_with_an_empty_dictionary() {
    let cfg = Config::load().expect("config");
    let f = compress_frame(&[b"retry me"], &Prefs::default());
    let calls = Cell::new(0u32);
    let saw_empty = Cell::new(false);
    let run = |input: &[u8], o: &DecodeOptions| -> cleanlz4::Result<FrameOutput> {
        calls.set(calls.get() + 1);
        match &o.dictionary {
            None => Err(Error::DictionaryRequired),
            Some(Dictionary { bytes, id: None }) if bytes.is_empty() => {
                saw_empty.set(true);
                cleanlz4::decompress_frame_with(input, &DecodeOptions::default())
            }
            Some(_) => Err(Error::DictionaryMismatch),
        }
    };
    let (v, _) = compare_frame_using(&f, &cfg, &run);
    assert_eq!(v, Verdict::Agree { accepted: true });
    assert_eq!(calls.get(), 2);
    assert!(saw_empty.get());
}

#[test]
fn an_unlisted_disagreement_is_diverge() {
    let v = judge(
        &ours(Err(Error::TruncatedInput)),
        &Decoded::Accepted(b"x".to_vec()),
        100,
    );
    assert!(matches!(v, Verdict::Diverge(_)), "{v:?}");
    let v = judge(
        &ours(Ok(b"x".to_vec())),
        &Decoded::Rejected("whatever".into()),
        100,
    );
    assert!(matches!(v, Verdict::Diverge(_)), "{v:?}");
    // A legacy frame is a named normalization, but only with the Legacy kind.
    let legacy = Ours {
        result: Ok(b"x".to_vec()),
        kinds: vec![FrameKind::Legacy],
    };
    let v = judge(&legacy, &Decoded::Rejected("unknown".into()), 100);
    assert_eq!(v, Verdict::Agree { accepted: true });
    // Over-cap on either side is skipped, not compared.
    let v = judge(&ours(Ok(b"x".to_vec())), &Decoded::OverCap, 100);
    assert_eq!(v, Verdict::Skipped("output-cap"));
    let v = judge(
        &ours(Err(Error::OutputLimitExceeded)),
        &Decoded::Accepted(vec![]),
        100,
    );
    assert_eq!(v, Verdict::Skipped("output-cap"));
}

fn seq(lits: &[u8], offset: u16, len: usize) -> Seq {
    Seq {
        literals: lits.to_vec(),
        offset,
        match_len: len,
    }
}

#[test]
fn compare_block_agrees_on_the_p1_06_probes() {
    let mut probes: Vec<(String, Vec<u8>, usize)> =
        vec![("E0".into(), vec![], 0), ("E1".into(), vec![0], 0)];
    let s = [seq(b"abcdefgh", 4, 16)];
    probes.push((
        "R1".into(),
        build_block_without_final(&s),
        expected_output(&s, b"").len(),
    ));
    for f in 0..=5usize {
        let fin: Vec<u8> = (0..f).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 16)];
        probes.push((
            format!("R2/{f}"),
            build_block(&s, &fin, 0),
            expected_output(&s, &fin).len(),
        ));
    }
    for d in 5..=12usize {
        let fin: Vec<u8> = (0..d - 4).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 4)];
        probes.push((
            format!("R3/{d}"),
            build_block(&s, &fin, 0),
            expected_output(&s, &fin).len(),
        ));
    }
    for n in [1u8, 7, 15] {
        probes.push((format!("N1/{n}"), build_block(&[], b"hello world", n), 11));
    }
    for (id, bytes, len) in probes {
        let v = compare_block(&bytes, len);
        assert!(matches!(v, Verdict::Agree { .. }), "{id}: {v:?}");
    }
    // A compressed real block, at its exact length.
    let data = word_salad(1, 10_000);
    let b = oracle::block::compress_default(&data);
    assert_eq!(
        compare_block(&b, data.len()),
        Verdict::Agree { accepted: true }
    );
    assert_eq!(compare_block(&b, 100), Verdict::Skipped("output-cap"));
}

/// A frame of one compressed block, independent, 64 KB maximum.
fn one_block_frame(block: &[u8]) -> Vec<u8> {
    let desc = [0x60u8, 0x40];
    let mut f = vec![0x04, 0x22, 0x4D, 0x18];
    f.extend_from_slice(&desc);
    f.push(((xxh32(&desc, 0) >> 8) & 0xFF) as u8);
    f.extend_from_slice(&(block.len() as u32).to_le_bytes());
    f.extend_from_slice(block);
    f.extend_from_slice(&[0, 0, 0, 0]);
    f
}

#[test]
fn zero_offset_block_is_agree_through_its_named_normalization() {
    let cfg = Config::load().expect("config");
    let s = [seq(b"abcdefgh", 0, 8)];
    let block = build_block(&s, b"12345", 0);
    let cap = 1000;
    // The oracle accepts it; cleanlz4 names it a zero offset.
    assert!(oracle::block::decompress_safe(&block, cap).is_ok());
    assert_eq!(
        cleanlz4::decompress_block(&block, cap),
        Err(Error::ZeroOffset)
    );
    assert_eq!(
        compare_block(&block, cap),
        Verdict::Agree { accepted: false }
    );
    let theirs = Decoded::Accepted(vec![]);
    assert_eq!(
        normalization_for(&ours(Err(Error::ZeroOffset)), &theirs),
        Some("zero-offset")
    );
    let frame = one_block_frame(&block);
    let (v, norm) = compare_frame_using(&frame, &cfg, &cleanlz4::decompress_frame_with);
    assert_eq!(v, Verdict::Agree { accepted: false });
    assert_eq!(norm, Some("zero-offset"));
}

#[test]
fn legacy_normalization_requires_a_legacy_frame() {
    let rejected = Decoded::Rejected("unknown".into());
    for kinds in [
        vec![FrameKind::Regular],
        vec![FrameKind::Skippable, FrameKind::Regular],
    ] {
        let o = Ours {
            result: Ok(b"x".to_vec()),
            kinds,
        };
        assert_eq!(normalization_for(&o, &rejected), None);
        let v = judge(&o, &rejected, 100);
        assert!(matches!(v, Verdict::Diverge(_)), "{v:?}");
    }
}

#[test]
fn compare_block_skips_when_only_the_oracle_goes_over_the_cap() {
    // A zero offset (cleanlz4 rejects it before the match is sized) and then a
    // long match: the oracle accepts and produces more than the cap.
    let s = [seq(b"abcdefgh", 0, 2000)];
    let block = build_block(&s, b"12345", 0);
    let cap = 100;
    assert_eq!(
        cleanlz4::decompress_block(&block, cap),
        Err(Error::ZeroOffset)
    );
    assert!(
        oracle::block::decompress_safe(&block, cap + oracle::diff::BLOCK_SLACK)
            .is_ok_and(|v| v.len() > cap)
    );
    assert_eq!(compare_block(&block, cap), Verdict::Skipped("output-cap"));
}
