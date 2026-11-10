use oracle::block::decompress_safe;
use oracle::craft::{build_block, expected_output, Seq};

fn seq(lits: &[u8], offset: u16, len: usize) -> Seq {
    Seq {
        literals: lits.to_vec(),
        offset,
        match_len: len,
    }
}

#[test]
fn crafted_valid_block_decodes_with_the_oracle() {
    let seqs = [seq(b"abcd", 4, 12)];
    let fin = b"WXYZ!";
    let block = build_block(&seqs, fin, 0);
    let want = expected_output(&seqs, fin);
    assert_eq!(want, b"abcdabcdabcdabcdWXYZ!");
    assert_eq!(decompress_safe(&block, want.len()).expect("accepted"), want);
}

#[test]
fn crafted_length_extensions_roundtrip() {
    for lit_len in [14usize, 15, 16, 269, 270, 271, 600] {
        for match_len in [4usize, 18, 19, 20, 273, 274, 275, 1000] {
            let lits: Vec<u8> = (0..lit_len).map(|i| (i * 7 + 1) as u8).collect();
            let seqs = [seq(&lits, 1, match_len)];
            let fin = b"123456789012";
            let block = build_block(&seqs, fin, 0);
            let want = expected_output(&seqs, fin);
            assert_eq!(
                decompress_safe(&block, want.len()).expect("accepted"),
                want,
                "lit {lit_len} match {match_len}"
            );
        }
    }
}

#[test]
fn crafted_overlap_match_decodes_with_the_oracle() {
    for offset in [1u16, 2, 3, 5, 7] {
        let lits: Vec<u8> = (1..=8).collect();
        let seqs = [seq(&lits, offset, 40)];
        let fin = b"67890";
        let block = build_block(&seqs, fin, 0);
        let want = expected_output(&seqs, fin);
        assert_eq!(
            decompress_safe(&block, want.len()).expect("accepted"),
            want,
            "offset {offset}"
        );
    }
}
