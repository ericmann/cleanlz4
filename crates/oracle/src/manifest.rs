//! `testdata/MANIFEST.json`: the record of every download and tool version.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Lower-case hex SHA-256 of a byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = Sha256::digest(data);
    let mut s = String::with_capacity(64);
    for b in digest.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// The oracle library's crate name and version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Binding {
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub version: String,
}

/// Tool versions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tools {
    pub oracle_binding: Binding,
    pub lz4_cli: String,
    pub rustc: String,
    pub cargo: String,
    pub cargo_fuzz: String,
}

/// One file inside a corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
}

/// One downloaded corpus archive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Corpus {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
    pub fetched_at: String,
    pub files: Vec<FileEntry>,
}

/// One generated conformance file (SPEC section 5.4's corpus record).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub suite: String,
    pub category: String,
    pub kind: String,
    pub tool: String,
    pub input: String,
    pub input_sha256: String,
    pub input_bytes: u64,
    pub params: serde_json::Value,
    /// Path relative to `testdata/conformance/`.
    pub file: String,
    pub file_sha256: String,
    pub file_bytes: u64,
    /// Suite-specific keys added by later tasks.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The generated conformance corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Conformance {
    pub seed: u64,
    pub corpus_sha256: String,
    pub entries: Vec<Entry>,
}

impl Conformance {
    /// Sort the entries by id and recompute `corpus_sha256`: the SHA-256 of
    /// every `id`, a newline, `file_sha256`, a newline, in that order.
    pub fn finalize(&mut self) {
        self.entries.sort_by(|a, b| a.id.cmp(&b.id));
        let mut text = String::new();
        for e in &self.entries {
            text.push_str(&e.id);
            text.push('\n');
            text.push_str(&e.file_sha256);
            text.push('\n');
        }
        self.corpus_sha256 = sha256_hex(text.as_bytes());
    }
}

/// The manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub version: u32,
    pub tools: Tools,
    pub corpora: Vec<Corpus>,
    pub conformance: Option<Conformance>,
}

/// Path of `testdata/MANIFEST.json`.
pub fn manifest_path() -> PathBuf {
    crate::config::repo_root()
        .join("testdata")
        .join("MANIFEST.json")
}

impl Manifest {
    /// Read the manifest from disk.
    pub fn load() -> Result<Manifest, String> {
        let p = manifest_path();
        let text = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Write the manifest, pretty-printed with a trailing newline.
    pub fn save(&self) -> Result<(), String> {
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        let p = manifest_path();
        std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))
    }
}

fn first_line_of(cmd: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("{cmd}: {e}"))?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    text.lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty())
        .map(|l| l.to_string())
        .ok_or_else(|| format!("{cmd}: no output"))
}

/// Version of the binding crate recorded in `Cargo.lock`.
pub fn binding_version_from_lock(lock: &str, name: &str) -> Option<String> {
    let mut lines = lock.lines();
    while let Some(l) = lines.next() {
        if l.trim() == format!("name = \"{name}\"") {
            let v = lines.next()?;
            let v = v.trim().strip_prefix("version = \"")?;
            return Some(v.trim_end_matches('"').to_string());
        }
    }
    None
}

/// Collect the tool versions.
pub fn collect_tools() -> Result<Tools, String> {
    let lock_path = crate::config::repo_root().join("Cargo.lock");
    let lock = std::fs::read_to_string(&lock_path).map_err(|e| e.to_string())?;
    let name = crate::ffi::BINDING_CRATE;
    let version = binding_version_from_lock(&lock, name)
        .ok_or_else(|| format!("{name} not found in Cargo.lock"))?;
    Ok(Tools {
        oracle_binding: Binding {
            crate_name: name.to_string(),
            version,
        },
        lz4_cli: first_line_of("lz4", &["-V"])?,
        rustc: first_line_of("rustc", &["--version"])?,
        cargo: first_line_of("cargo", &["--version"])?,
        cargo_fuzz: first_line_of("cargo", &["fuzz", "--version"])?,
    })
}

fn hash_file(p: &Path) -> Result<(String, u64), String> {
    let data = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
    Ok((sha256_hex(&data), data.len() as u64))
}

fn list_files(dir: &Path) -> Result<Vec<FileEntry>, String> {
    let mut names = Vec::new();
    let rd = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| e.to_string())?;
        if ent.path().is_file() {
            names.push(ent.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    let mut files = Vec::new();
    for n in names {
        let (sha256, bytes) = hash_file(&dir.join(&n))?;
        files.push(FileEntry {
            name: n,
            sha256,
            bytes,
        });
    }
    Ok(files)
}

/// Build the corpora section from `testdata/corpora/`.
pub fn build_corpora(fetched_at: &str) -> Result<Vec<Corpus>, String> {
    let base = crate::config::repo_root().join("testdata").join("corpora");
    let specs = [
        (
            "canterbury",
            "cantrbry.tar.gz",
            "cantrbry",
            "https://corpus.canterbury.ac.nz/resources/cantrbry.tar.gz",
        ),
        (
            "silesia",
            "silesia.zip",
            "silesia",
            "https://sun.aei.polsl.pl/~sdeor/corpus/silesia.zip",
        ),
    ];
    let mut out = Vec::new();
    for (name, archive, dir, url) in specs {
        let (sha256, bytes) = hash_file(&base.join(archive))?;
        out.push(Corpus {
            name: name.to_string(),
            url: url.to_string(),
            sha256,
            bytes,
            fetched_at: fetched_at.to_string(),
            files: list_files(&base.join(dir))?,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_of_known_bytes() {
        // The hash of the empty input is defined by FIPS 180-4.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let m = Manifest {
            version: 1,
            tools: Tools {
                oracle_binding: Binding {
                    crate_name: "x".into(),
                    version: "1".into(),
                },
                lz4_cli: "a".into(),
                rustc: "b".into(),
                cargo: "c".into(),
                cargo_fuzz: "d".into(),
            },
            corpora: vec![Corpus {
                name: "n".into(),
                url: "u".into(),
                sha256: sha256_hex(b"a"),
                bytes: 1,
                fetched_at: "t".into(),
                files: vec![FileEntry {
                    name: "f".into(),
                    sha256: sha256_hex(b"b"),
                    bytes: 2,
                }],
            }],
            conformance: None,
        };
        let text = serde_json::to_string_pretty(&m).expect("serialize");
        let back: Manifest = serde_json::from_str(&text).expect("parse");
        assert_eq!(m, back);
    }

    #[test]
    fn lock_version_lookup() {
        let lock = "[[package]]\nname = \"abc\"\nversion = \"1.2.3+x\"\n";
        assert_eq!(
            binding_version_from_lock(lock, "abc").as_deref(),
            Some("1.2.3+x")
        );
    }
}
