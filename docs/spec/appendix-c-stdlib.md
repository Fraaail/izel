# Appendix C: Standard Library API (Normative)

This appendix maps language-level standard library wards to v0.1 baseline guarantees.

Reference modules live under `library/std/` and are regression-checked by `crates/izel_std` tests.

Coverage includes:
- Core, collections, I/O+OS, concurrency, math/hash/codec, testing.

## Conformance Expectations

- Declared standard modules MUST remain parseable and resolvable by toolchain front-end phases.
- Regressions in required symbol surfaces SHOULD be guarded by automated tests.

## Baseline Module Groups

- Core: `prim`, `ops`, `cmp`, `iter`, `option`, `result`, `fmt`, `mem`, `ptr`, `slice`, `str`,
  `range`, `marker`.
- System: `io`, `fs`, `path`, `env`, `os`, `ffi`.
- Concurrency: `thread`, `sync`, `atomic`, `chan`, `async`.
- Data and codec: `collections`, `hash`, `crypt`, `codec`, `json`.
- Testing: `test`, `bench`, `mock`.

## Evolution Policy

New modules MAY be added in minor revisions. Removal or incompatible renaming of baseline modules
requires edition-level migration strategy.
