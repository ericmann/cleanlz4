# Design notes

## Decisions

- Lint attributes use the `cfg_attr(not(test), deny(...))` form in `crates/cleanlz4/src/lib.rs`, so the no-panic lints bind library code while tests may use `unwrap`, `expect` and indexing (SPEC section 3.3). Implements: project rule on no reachable panic; held by the `no-panic-lint-suppression` constraint and by `make check` (clippy).

## Spec rules implemented

## Differential rules

## Divergence triage

## Measurements
