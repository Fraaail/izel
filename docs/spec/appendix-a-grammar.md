# Appendix A: Grammar Reference (Normative)

This appendix defines grammar reference conventions for v1.0.

The implementation parser and CST forms are the conformance source for grammar details.
A canonical EBNF extract is maintained alongside parser evolution.

## Grammar Source Of Truth

- Parser implementation under `crates/izel_parser`.
- CST node model under `crates/izel_parser/src/cst.rs`.

## Operator Precedence

Operator precedence and associativity are implementation-defined in parser tables and MUST be
stable within a language edition.

## Recovery

Grammar conformance includes resilient recovery behavior:
- malformed productions SHOULD surface diagnostics,
- parser progress MUST continue where feasible,
- implementations MUST avoid panic for regular malformed source inputs.

Core references:
- [02-syntax.md](02-syntax.md)
- parser implementation in `crates/izel_parser`
