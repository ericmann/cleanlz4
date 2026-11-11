//! The release is consistent: both crates carry one version, and the newest
//! changelog entry and its link line name it.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The `version` line of a manifest's `[package]` section.
fn package_version(manifest: &str) -> String {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package {
            if let Some(rest) = line.strip_prefix("version") {
                let value = rest
                    .trim_start()
                    .strip_prefix('=')
                    .expect("version =")
                    .trim();
                return value.trim_matches('"').to_string();
            }
        }
    }
    panic!("no version in [package]");
}

fn crate_version() -> String {
    let a = package_version(&read("crates/cleanlz4/Cargo.toml"));
    let b = package_version(&read("crates/oracle/Cargo.toml"));
    assert_eq!(a, b, "the two crates must carry one version");
    a
}

#[test]
fn crate_versions_agree() {
    assert!(!crate_version().is_empty());
}

#[test]
fn changelog_newest_entry_is_the_crate_version() {
    let version = crate_version();
    let changelog = read("CHANGELOG.md");
    let heading = changelog
        .lines()
        .find(|l| l.starts_with("## ["))
        .expect("a changelog entry");
    let prefix = format!("## [{version}] - ");
    let date = heading
        .strip_prefix(&prefix)
        .unwrap_or_else(|| panic!("newest heading {heading:?} is not for {version}"));
    let parts: Vec<&str> = date.split('-').collect();
    let widths: Vec<usize> = parts.iter().map(|p| p.len()).collect();
    assert_eq!(widths, [4, 2, 2], "date {date:?}");
    assert!(parts.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit())));
}

#[test]
fn changelog_links_the_crate_version() {
    let version = crate_version();
    let prefix = format!("[{version}]: ");
    assert!(
        read("CHANGELOG.md").lines().any(|l| l.starts_with(&prefix)),
        "no line starts with {prefix:?}"
    );
}
