#!/usr/bin/env bash
# Usage: check-deps.sh [root]. Fails on a dependency, an unsafe token or a missing forbid in crates/cleanlz4.
root="${1:-.}"
dir="$root/crates/cleanlz4"
fail=0
if [ ! -f "$dir/Cargo.toml" ]; then echo "missing $dir/Cargo.toml"; exit 1; fi
in_dep=0
while IFS= read -r line || [ -n "$line" ]; do
  t="${line%%#*}"
  t="$(echo "$t" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
  [ -z "$t" ] && continue
  case "$t" in
    \[*dependencies*\]) 
      in_dep=1
      if [ "$t" != "[dependencies]" ]; then echo "extra dependency table: $t"; fail=1; fi
      continue;;
    \[*) in_dep=0; continue;;
  esac
  if [ "$in_dep" = 1 ]; then echo "dependency entry: $t"; fail=1; fi
done < "$dir/Cargo.toml"
if grep -rnw unsafe "$dir" --include='*' --exclude-dir=target; then echo "unsafe token found"; fail=1; fi
if ! grep -q '^#!\[forbid(unsafe_code)\]' "$dir/src/lib.rs" 2>/dev/null; then echo "lib.rs lacks forbid(unsafe_code)"; fail=1; fi
exit $fail
