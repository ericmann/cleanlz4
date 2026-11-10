use cleanlz4::{parse_frame_descriptor, BlockMaxSize};
use oracle::frame::{compress_frame, Prefs};

#[test]
fn descriptor_matches_oracle_prefs_for_every_combination() {
    let data = b"descriptor test data, descriptor test data, descriptor test data".to_vec();
    let mut n = 0;
    for id in 4u8..=7 {
        for linked in [false, true] {
            for block_checksum in [false, true] {
                for content_checksum in [false, true] {
                    for size in [None, Some(data.len() as u64)] {
                        for dict_id in [0u32, 0xDEAD_BEEF] {
                            let prefs = Prefs {
                                block_size_id: id,
                                linked,
                                content_checksum,
                                block_checksum,
                                content_size: size,
                                dict_id,
                                ..Prefs::default()
                            };
                            let f = compress_frame(&[&data], &prefs);
                            let (d, len) = parse_frame_descriptor(&f)
                                .unwrap_or_else(|e| panic!("{prefs:?}: {e}"));
                            let want = match id {
                                4 => BlockMaxSize::Max64KB,
                                5 => BlockMaxSize::Max256KB,
                                6 => BlockMaxSize::Max1MB,
                                _ => BlockMaxSize::Max4MB,
                            };
                            assert_eq!(d.block_max_size, want, "{prefs:?}");
                            assert_eq!(d.block_independence, !linked, "{prefs:?}");
                            assert_eq!(d.block_checksum, block_checksum, "{prefs:?}");
                            assert_eq!(d.content_checksum, content_checksum, "{prefs:?}");
                            assert_eq!(d.content_size, size, "{prefs:?}");
                            let want_dict = if dict_id == 0 { None } else { Some(dict_id) };
                            assert_eq!(d.dict_id, want_dict, "{prefs:?}");
                            let expect_len = 7
                                + if size.is_some() { 8 } else { 0 }
                                + if want_dict.is_some() { 4 } else { 0 };
                            assert_eq!(len, expect_len, "{prefs:?}");
                            n += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(n, 4 * 2 * 2 * 2 * 2 * 2);
}
