GUARD = node /home/ericmann/.claude/plugins/cache/displacetech/kukulu/0.8.0/scripts/cleanroom-guard.mjs --self-test docs/cleanroom.json

.PHONY: check
check:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	$(GUARD)
	bash scripts/check-deps.sh
	bash scripts/test-check-deps.sh
	node scripts/verify-spec-sources.mjs
	node scripts/test-verify-spec-sources.mjs

.PHONY: manifest-corpora
manifest-corpora:
	cargo run -p oracle --bin manifest -- corpora --fetched-at $(FETCHED_AT)

.PHONY: corpus
corpus:
	cargo run --release -p oracle --bin gen_corpus -- --suites all

.PHONY: conformance conformance-check
conformance:
	cargo run --release -p oracle --bin conformance -- --task $(TASK) --categories $(CATEGORIES)

conformance-check:
	cargo run --release -p oracle --bin conformance -- --task check --categories all --check

.PHONY: bench
bench:
	cargo run --release -p oracle --example bench_decode -- --task $(TASK)
