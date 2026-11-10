use cleanlz4::{decompress_frame_with, DecodeOptions, Dictionary, Error};
use oracle::config::Config;
use oracle::dict::dict_frame;
use oracle::synth::word_salad;

fn opts(dict: &[u8], id: Option<u32>) -> DecodeOptions {
    DecodeOptions {
        dictionary: Some(Dictionary {
            bytes: dict.to_vec(),
            id,
        }),
        ..DecodeOptions::default()
    }
}

/// Chunks that share text with the dictionary, so blocks reach into it.
fn chunks_for(dict: &[u8], seed: u64) -> Vec<Vec<u8>> {
    let n = dict.len();
    let mut c1 = dict[n - 3000..].to_vec();
    c1.extend_from_slice(&word_salad(seed, 5000));
    let mut c2 = dict[n / 2..n / 2 + 2000].to_vec();
    c2.extend_from_slice(&c1[..1000]);
    let c3 = word_salad(seed + 7, 70_000);
    vec![c1, c2, c3]
}

fn check(independent: bool) {
    let cfg = Config::load().expect("config");
    for size in &cfg.dict.sizes_bytes {
        let dict = word_salad(cfg.fuzz.seed, *size as usize);
        for id in [None, Some(0xD1C7_u32)] {
            let chunks = chunks_for(&dict, cfg.fuzz.seed);
            let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
            let expected: Vec<u8> = chunks.concat();
            let frame = dict_frame(&dict, &refs, independent, id);
            let out = decompress_frame_with(&frame, &opts(&dict, id)).expect("decodes");
            assert_eq!(out.data, expected, "size {size} id {id:?}");
        }
    }
}

#[test]
fn assembled_linked_dictionary_frames_decode() {
    check(false);
}

#[test]
fn assembled_independent_dictionary_frames_decode() {
    check(true);
}

#[test]
fn the_wrong_dictionary_gives_an_error_or_other_bytes_and_never_panics() {
    let cfg = Config::load().expect("config");
    for independent in [false, true] {
        for size in &cfg.dict.sizes_bytes {
            let dict = word_salad(cfg.fuzz.seed, *size as usize);
            let chunks = chunks_for(&dict, cfg.fuzz.seed);
            let refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
            let expected: Vec<u8> = chunks.concat();
            let frame = dict_frame(&dict, &refs, independent, None);
            let inverted: Vec<u8> = dict.iter().map(|b| !b).collect();
            match decompress_frame_with(&frame, &opts(&inverted, None)) {
                Ok(o) => assert_ne!(o.data, expected),
                Err(e) => assert_eq!(e, Error::OffsetOutOfRange),
            }
            // A frame with a Dict-ID and no dictionary is refused.
            let with_id = dict_frame(&dict, &refs, independent, Some(5));
            assert_eq!(
                decompress_frame_with(&with_id, &DecodeOptions::default()).map(|o| o.data),
                Err(Error::DictionaryRequired)
            );
        }
    }
}
