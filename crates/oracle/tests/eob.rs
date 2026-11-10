use cleanlz4::decompress_block;
use oracle::block::{compress_default, decompress_safe};
use oracle::craft::{build_block, build_block_without_final, expected_output, Seq};

const SLACK: usize = 65_536;

fn seq(lits: &[u8], offset: u16, len: usize) -> Seq {
    Seq {
        literals: lits.to_vec(),
        offset,
        match_len: len,
    }
}

fn probes() -> Vec<(String, Vec<u8>, usize)> {
    let mut v: Vec<(String, Vec<u8>, usize)> = Vec::new();
    v.push(("E0".into(), vec![], 0));
    v.push(("E1".into(), vec![0], 0));
    let s = [seq(b"abcdefgh", 4, 16)];
    v.push((
        "R1".into(),
        build_block_without_final(&s),
        expected_output(&s, b"").len(),
    ));
    for f in 0..=5usize {
        let fin: Vec<u8> = (0..f).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 16)];
        v.push((
            format!("R2/{f}"),
            build_block(&s, &fin, 0),
            expected_output(&s, &fin).len(),
        ));
    }
    for d in 5..=12usize {
        let fin: Vec<u8> = (0..d - 4).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 4)];
        v.push((
            format!("R3/{d}"),
            build_block(&s, &fin, 0),
            expected_output(&s, &fin).len(),
        ));
    }
    for n in [1u8, 7, 15] {
        v.push((format!("N1/{n}"), build_block(&[], b"hello world", n), 11));
    }
    for total in [11usize, 12] {
        let lits: Vec<u8> = (0..total - 9).map(|i| b'a' + i as u8).collect();
        let s = [seq(&lits, 1, 4)];
        v.push((
            format!("L1/{total}"),
            build_block(&s, b"12345", 0),
            expected_output(&s, b"12345").len(),
        ));
    }
    v
}

/// Compare both decoders on `block` with `cap` as the capacity; `what` names the case.
fn agree(block: &[u8], cap: usize, what: &str) {
    let ours = decompress_block(block, cap);
    let theirs = decompress_safe(block, cap);
    match (ours, theirs) {
        (Ok(a), Ok(b)) => assert_eq!(a, b, "{what}: outputs differ"),
        (Err(_), Err(_)) => {}
        // Differential rule: the block format calls a zero offset invalid
        // ("denotes an invalid (corrupted) block"); the oracle tolerates it
        // and cleanlz4 stays strict. See DESIGN.md, Differential rules.
        (Err(cleanlz4::Error::ZeroOffset), Ok(_)) => {}
        (a, b) => panic!("{what}: cleanlz4 {a:?} vs oracle {:?}", b.map(|v| v.len())),
    }
}

#[test]
fn end_of_block_probes_agree_with_the_oracle() {
    for (id, bytes, len) in probes() {
        agree(&bytes, len + SLACK, &id);
    }
}

fn small_blocks() -> Vec<(Vec<u8>, Vec<u8>)> {
    let a = b"the quick brown fox jumps over the lazy dog; the quick brown fox jumps again and again and again".to_vec();
    let b: Vec<u8> = (0..150u32).map(|i| (i % 7) as u8 + b'a').collect();
    let c = b"abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabc-end-of-the-line"
        .to_vec();
    [a, b, c]
        .into_iter()
        .map(|d| (compress_default(&d), d))
        .collect()
}

#[test]
fn every_prefix_of_small_oracle_blocks_agrees() {
    for (block, original) in small_blocks() {
        assert!(block.len() < 200);
        for cut in 0..=block.len() {
            agree(
                &block[..cut],
                original.len() + SLACK,
                &format!("prefix {cut} of {}", block.len()),
            );
        }
    }
}

#[test]
fn single_byte_corruptions_of_small_oracle_blocks_agree() {
    for (block, original) in small_blocks() {
        for pos in 0..block.len() {
            for value in [0x00u8, 0xFF, 0x5A] {
                let mut bad = block.clone();
                bad[pos] = value;
                agree(
                    &bad,
                    original.len() + SLACK,
                    &format!("pos {pos} value {value:#x}"),
                );
            }
        }
    }
}
