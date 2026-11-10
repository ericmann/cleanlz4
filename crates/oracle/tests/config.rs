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

fn numeric_leaves(v: &serde_json::Value, out: &mut Vec<u64>) {
    match v {
        serde_json::Value::Number(n) => {
            if let Some(x) = n.as_u64() {
                out.push(x);
            }
        }
        serde_json::Value::Array(a) => a.iter().for_each(|x| numeric_leaves(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| numeric_leaves(x, out)),
        _ => {}
    }
}

fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn grouped(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(c);
    }
    out
}

#[test]
fn large_config_values_are_not_spelled_in_oracle_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("../../testdata/config.json")).expect("config");
    let json: serde_json::Value = serde_json::from_str(&text).expect("json");
    let mut leaves = Vec::new();
    numeric_leaves(&json, &mut leaves);
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("examples"), &mut files);
    let mut hits = Vec::new();
    for f in files {
        if f.ends_with("src/config.rs") {
            continue;
        }
        let src = std::fs::read_to_string(&f).expect("source");
        for n in leaves.iter().copied().filter(|n| *n >= 1_000_000) {
            let mut forms = vec![n.to_string(), grouped(n)];
            if n.is_power_of_two() {
                let k = n.trailing_zeros();
                forms.push(format!("1 << {k}"));
                forms.push(format!("1<<{k}"));
            }
            for form in forms {
                if src.contains(&form) {
                    hits.push(format!("{}: {form}", f.display()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "config values spelled in sources: {hits:?}"
    );
}
