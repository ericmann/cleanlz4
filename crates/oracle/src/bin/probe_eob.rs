//! Spike P1-06: which end-of-block violations does the oracle's block decoder reject?

use oracle::block::decompress_safe;
use oracle::craft::{build_block, build_block_without_final, expected_output, Seq};

struct Probe {
    id: String,
    bytes: Vec<u8>,
    expected_len: usize,
}

fn seq(lits: &[u8], offset: u16, len: usize) -> Seq {
    Seq {
        literals: lits.to_vec(),
        offset,
        match_len: len,
    }
}

fn hex(b: &[u8]) -> String {
    if b.is_empty() {
        return String::from("(empty)");
    }
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn verdict(bytes: &[u8], capacity: usize, expected_len: usize) -> String {
    match decompress_safe(bytes, capacity) {
        Ok(v) if v.len() == expected_len => String::from("accept"),
        Ok(v) => format!("accept (len {})", v.len()),
        Err(_) => String::from("reject"),
    }
}

fn probes() -> Vec<Probe> {
    let mut v = Vec::new();
    v.push(Probe {
        id: "E0".into(),
        bytes: vec![],
        expected_len: 0,
    });
    v.push(Probe {
        id: "E1".into(),
        bytes: vec![0],
        expected_len: 0,
    });
    // R1: a block ending right after a match.
    let s = [seq(b"abcdefgh", 4, 16)];
    v.push(Probe {
        id: "R1".into(),
        bytes: build_block_without_final(&s),
        expected_len: expected_output(&s, b"").len(),
    });
    // R2: a match followed by 0..=5 final literals.
    for f in 0..=5usize {
        let fin: Vec<u8> = (0..f).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 16)];
        v.push(Probe {
            id: format!("R2/{f}"),
            bytes: build_block(&s, &fin, 0),
            expected_len: expected_output(&s, &fin).len(),
        });
    }
    // R3: the last match's copy begins d bytes before the end; match length 4.
    for d in 5..=12usize {
        let f = d - 4;
        let fin: Vec<u8> = (0..f).map(|i| b'0' + i as u8).collect();
        let s = [seq(b"abcdefgh", 8, 4)];
        v.push(Probe {
            id: format!("R3/{d}"),
            bytes: build_block(&s, &fin, 0),
            expected_len: expected_output(&s, &fin).len(),
        });
    }
    // N1: a literal-only block with a non-zero low nibble in the final token.
    for n in [1u8, 7, 15] {
        v.push(Probe {
            id: format!("N1/{n}"),
            bytes: build_block(&[], b"hello world", n),
            expected_len: 11,
        });
    }
    // L1: one match, 11 and 12 bytes in total.
    for total in [11usize, 12] {
        let lit = total - 4 - 5;
        let lits: Vec<u8> = (0..lit).map(|i| b'a' + i as u8).collect();
        let s = [seq(&lits, 1, 4)];
        let fin = b"12345";
        v.push(Probe {
            id: format!("L1/{total}"),
            bytes: build_block(&s, fin, 0),
            expected_len: expected_output(&s, fin).len(),
        });
    }
    v
}

fn main() {
    println!("| probe | bytes (hex) | exact capacity | with 65536 slack |");
    println!("|---|---|---|---|");
    for p in probes() {
        let exact = verdict(&p.bytes, p.expected_len, p.expected_len);
        let slack = verdict(&p.bytes, p.expected_len + 65_536, p.expected_len);
        println!("| {} | `{}` | {} | {} |", p.id, hex(&p.bytes), exact, slack);
    }
}
