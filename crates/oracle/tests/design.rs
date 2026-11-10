use oracle::config::repo_root;

#[test]
fn design_report_cites_each_phase_1_section() {
    let text =
        std::fs::read_to_string(repo_root().join("docs").join("DESIGN.md")).expect("DESIGN.md");
    for needle in [
        "XXH32 Algorithm Description",
        "Compressed block format",
        "End of block conditions",
        "Safe decoding",
    ] {
        assert!(text.contains(needle), "missing {needle}");
    }
    for step in 1..=7 {
        assert!(
            text.contains(&format!("Step {step}")),
            "missing Step {step}"
        );
    }
}
