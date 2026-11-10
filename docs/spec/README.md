# docs/spec

The only implementation-adjacent inputs this build may read (SPEC §2). Each file is a byte copy from the upstream repository at a pinned commit. `SOURCES.json` records the repository, commit, path, raw URL, SHA-256 and size of each file.

| File | What it specifies | Used for |
|---|---|---|
| `lz4_Block_format.md` | The LZ4 block encoding: sequences, tokens, literals, matches, offsets, end-of-block rules | The block decoder |
| `lz4_Frame_format.md` | The LZ4 frame: magic numbers, frame descriptor, block sizes, linked and independent blocks, block and content checksums, content size, dictionary ID, skippable frames, legacy frame | The frame decoder |
| `xxhash_spec.md` | The xxHash algorithms | **XXH32 only.** The frame's header, block and content checksums use XXH32. XXH64 and XXH3 are out of scope |

There is no IETF RFC for LZ4. These documents are the format's published specification, written by its author for implementers in any language.

`xxhash_spec.md` contains illustrative code fragments as part of describing the algorithm. They are part of the specification and are permitted input, like the sample code inside RFC 1950 and RFC 1952 was for cleanflate. The xxHash *implementation* (`xxhash.h`, `xxhash.c`) is forbidden.

Licensing notices:

- `lz4_Frame_format.md` and `xxhash_spec.md` carry Yann Collet's notice permitting copying and distribution with the notice preserved. Their copies here are unmodified.
- `lz4_Block_format.md` carries no notice of its own; under the upstream repository's default it is GPL-2.0-or-later. It is included unmodified as a separate document.

A task that needs a fact not in these documents blocks and asks. It never looks the fact up in an implementation.
