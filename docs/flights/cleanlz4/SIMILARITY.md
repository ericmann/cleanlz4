# Contamination score

The subject (crates/cleanlz4/src) was compared with 2 test pairs (subject-vs-c, subject-vs-indep) and placed between a negative control (independent-vs-c) and a positive control (derived-vs-c). A test pair at or below the negative control on a metric that separates the controls shows no similarity beyond independent work.

## Positioning

- subject-vs-c, ngram_jaccard@5: 0.0352 (c = -0.3999, below negative)
- subject-vs-c, ngram_containment@5: 0.1137 (c = 0.2128, at or near negative)
- subject-vs-c, ngram_jaccard@10: 0.0006 (c = -0.2180, below negative)
- subject-vs-c, ngram_containment@10: 0.0029 (c = -0.3183, below negative)
- subject-vs-c, ident_jaccard: 0.0128 (c = 0.0071, at or near negative)
- subject-vs-c, ident_match_rate_left: 0.0687 (c = 0.0477, at or near negative)
- subject-vs-c, comment_jaccard: 0.0001 (c = n/a, not separated)
- subject-vs-indep, ngram_jaccard@5: 0.2231 (c = 7.4416, above positive)
- subject-vs-indep, ngram_containment@5: 0.4805 (c = 8.5513, above positive)
- subject-vs-indep, ngram_jaccard@10: 0.0523 (c = 8.7797, above positive)
- subject-vs-indep, ngram_containment@10: 0.1324 (c = 12.2093, above positive)
- subject-vs-indep, ident_jaccard: 0.0793 (c = 0.1058, at or near negative)
- subject-vs-indep, ident_match_rate_left: 0.1924 (c = 0.1990, at or near negative)
- subject-vs-indep, comment_jaccard: 0.0007 (c = n/a, not separated)

## Verdict

independence not supported: subject-vs-indep: ngram_jaccard@5 is above positive; subject-vs-indep: ngram_containment@5 is above positive; subject-vs-indep: ngram_jaccard@10 is above positive; subject-vs-indep: ngram_containment@10 is above positive

## Raw scores

| pair | role | ngram_jaccard@5 | ngram_containment@5 | ngram_jaccard@10 | ngram_containment@10 | ident_jaccard | ident_match_rate_left | comment_jaccard |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| independent-vs-c | negative | 0.0448 | 0.1043 | 0.0018 | 0.0062 | 0.0080 | 0.0297 | 0.0003 |
| derived-vs-c | positive | 0.0687 | 0.1483 | 0.0076 | 0.0165 | 0.6820 | 0.8475 | 0.0000 |
| subject-vs-c | test | 0.0352 | 0.1137 | 0.0006 | 0.0029 | 0.0128 | 0.0687 | 0.0001 |
| subject-vs-indep | test | 0.2231 | 0.4805 | 0.0523 | 0.1324 | 0.0793 | 0.1924 | 0.0007 |

## Limits

- This is a lexical approximation of cleanflate-bench's tree-sitter token method: the same normalization but no parse tree, so a C macro body and a Rust impl-for block are handled by rule rather than by grammar.
- There is no clone detector: copied logic that was renamed and reordered below the n-gram window is not found.
- Cross-language pairs score lower on token metrics by construction, as the bench found: its n-gram metrics did not separate its controls, and its identifier and comment metrics did.
- A structural metric needs a same-language positive control.
- One target library is n = 1: a score is evidence about this subject and this prior art, not a general property.
