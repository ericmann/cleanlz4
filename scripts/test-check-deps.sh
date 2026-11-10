#!/usr/bin/env bash
# Builds four temporary trees and checks check-deps.sh's exit status on each.
set -u
here="$(cd "$(dirname "$0")" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
status=0
mk() {
  d="$tmp/$1"; mkdir -p "$d/crates/cleanlz4/src"
  printf '[package]\nname = "cleanlz4"\n\n[dependencies]\n' > "$d/crates/cleanlz4/Cargo.toml"
  printf '#![forbid(unsafe_code)]\n' > "$d/crates/cleanlz4/src/lib.rs"
}
expect() { # name want
  bash "$here/check-deps.sh" "$tmp/$1" >/dev/null 2>&1; got=$?
  if { [ "$2" = 0 ] && [ $got -eq 0 ]; } || { [ "$2" = 1 ] && [ $got -ne 0 ]; }; then echo "ok: $1"; else echo "FAIL: $1 (exit $got)"; status=1; fi
}
mk clean; expect clean 0
mk dep; echo 'foo = "1"' >> "$tmp/dep/crates/cleanlz4/Cargo.toml"; expect dep 1
mk unsafe; echo 'fn f() { unsafe { } }' >> "$tmp/unsafe/crates/cleanlz4/src/lib.rs"; expect unsafe 1
mk noforbid; echo '// nothing' > "$tmp/noforbid/crates/cleanlz4/src/lib.rs"; expect noforbid 1
exit $status
