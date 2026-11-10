use oracle::config::Config;
use oracle::fuzzrun::{
    assemble_result, duration_seconds, fuzz_args, parse_final_stats, regression_name,
};

#[test]
fn arguments_come_from_the_config() {
    let cfg = Config::load().expect("config");
    let args = fuzz_args(&cfg, "no_panic_block", false);
    assert_eq!(
        &args[..5],
        ["+nightly", "fuzz", "run", "-a", "no_panic_block"]
    );
    assert_eq!(args[5], "fuzz/corpus/no_panic_block");
    assert!(args.contains(&format!("-max_total_time={}", cfg.fuzz.seconds_per_target)));
    assert!(args.contains(&format!("-seed={}", cfg.fuzz.seed)));
    assert!(args.contains(&"-print_final_stats=1".to_string()));
}

#[test]
fn final_stats_are_parsed() {
    let out =
        "#123 DONE\nstat::number_of_executed_units: 4567\nstat::average_exec_per_sec:     100\n";
    assert_eq!(parse_final_stats(out), Some(4567));
    assert_eq!(parse_final_stats("no stats here"), None);
    assert_eq!(parse_final_stats("stat::number_of_executed_units: x"), None);
}

#[test]
fn a_result_serializes_to_the_spec_schema() {
    let cfg = Config::load().expect("config");
    let r = assemble_result(&cfg, "P4-04", "no_panic_block", 12, 345, 1, 2, 6);
    let v: serde_json::Value = serde_json::to_value(&r).expect("json");
    let mut keys: Vec<&str> = v
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "corpus_entries",
            "crashes",
            "divergences",
            "runs",
            "seconds",
            "seed",
            "target",
            "task"
        ]
    );
    assert_eq!(v["seed"], cfg.fuzz.seed);
    assert_eq!(v["crashes"], 1);
    assert_eq!(
        regression_name("t", b"abc").len(),
        "t-".len() + 16 + ".bin".len()
    );
}

#[test]
fn smoke_uses_the_smoke_duration_key() {
    let cfg = Config::load().expect("config");
    assert_eq!(duration_seconds(&cfg, true), cfg.fuzz.smoke_seconds);
    assert_eq!(duration_seconds(&cfg, false), cfg.fuzz.seconds_per_target);
    let args = fuzz_args(&cfg, "diff_accept", true);
    assert!(args.contains(&format!("-max_total_time={}", cfg.fuzz.smoke_seconds)));
    assert!(!args.contains(&format!("-max_total_time={}", cfg.fuzz.seconds_per_target)));
}
