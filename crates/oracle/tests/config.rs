use oracle::config::Config;

#[test]
fn config_file_parses_with_every_key() {
    let c = Config::load().expect("config parses");
    assert!(!c.dict.sizes_bytes.is_empty());
    assert!(!c.conformance.hc_levels.is_empty());
    assert!(!c.conformance.frame_levels.is_empty());
}

#[test]
fn limits_default_matches_library_constant() {
    let c = Config::load().expect("config parses");
    assert_eq!(
        c.limits.max_output_bytes,
        cleanlz4::DEFAULT_MAX_OUTPUT_BYTES
    );
}

#[test]
fn tunable_values_are_positive() {
    let c = Config::load().expect("config parses");
    let nums = [
        c.limits.max_output_bytes,
        c.fuzz.seconds_per_target,
        c.fuzz.seed,
        c.fuzz.smoke_seconds,
        c.fuzz.max_output_bytes,
        c.fuzz.max_divergence_files,
        c.bench.repeats,
        c.stream.chunking_cases,
        c.conformance.silesia_combos_per_file,
        c.conformance.large_input_bytes,
        c.conformance.random_block_cases,
        c.diff.max_output_bytes,
    ];
    for n in nums {
        assert!(n > 0);
    }
    assert!(c.dict.sizes_bytes.iter().all(|&n| n > 0));
}
