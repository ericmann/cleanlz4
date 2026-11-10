use oracle::bench::{measure, rate_mb_s};
use oracle::frame::{compress_frame, Prefs};
use oracle::synth::word_salad;

#[test]
fn measure_reports_positive_rates_for_a_small_input() {
    let data = word_salad(1, 200_000);
    let prefs = Prefs {
        block_size_id: 7,
        content_checksum: true,
        ..Prefs::default()
    };
    let f = compress_frame(&[&data], &prefs);
    // One repeat is enough to check the plumbing; the config's count is the tool's business.
    let r = measure(&f, data.len(), 1);
    assert!(r.cleanlz4_mb_s > 0.0, "{r:?}");
    assert!(r.c_mb_s > 0.0, "{r:?}");
}

#[test]
fn rate_is_bytes_over_seconds_in_megabytes() {
    assert_eq!(rate_mb_s(1_000_000, 1.0), 1.0);
    assert_eq!(rate_mb_s(500_000_000, 2.0), 250.0);
    assert_eq!(rate_mb_s(123_456_789, 1.0), 123.5);
    assert_eq!(rate_mb_s(1_000_000, 0.0), 0.0);
}
