# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Expanded book chapters under `docs/book/` with complete introductory coverage.
- Expanded normative and compatibility content under `docs/spec/`.
- Added concrete compile-pass/compile-fail fixtures replacing empty keep-files.
- Added broader `izel_pm` parser and CLI branch coverage tests.

### Changed
- Replaced transitional dual round-trip test body generation with an empty, valid body.
- Type checker now records inferred expression types in `expr_types`.
- MIR codegen now emits LLVM phi handling instead of no-op fallback behavior.

