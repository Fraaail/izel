# 2. Syntax And Grammar (Normative)

## Parsing Model
- Parsing is deterministic and produces a concrete syntax tree (CST).
- CST lowering produces the abstract syntax tree (AST).
- Parsers MUST recover from common syntax errors and continue to discover additional diagnostics
	where practical.

## Top-Level Items
The following top-level declarations are part of v1.0:
- `forge`, `shape`, `scroll`, `weave`, `impl`, `ward`, `draw`, `echo`, `bridge`, `raw` blocks.

Unsupported top-level constructs MUST produce diagnostics rather than panics.

## Expressions
v1.0 includes:
- literals and identifiers,
- unary and binary operators,
- function calls,
- `given`/`else`, `branch`, loops,
- blocks and trailing expressions,
- pipelines with `|>`.

Expression parsing MUST respect precedence and associativity defined by implementation parser tables.

## Attributes
- Bracket attributes `#[...]` are valid on supported declarations.
- Unsupported placement MUST produce a diagnostic.

## Pattern And Control Forms
- `branch` matching forms are part of v1.0 control flow.
- `given` and `else` forms are expressions and statements depending on context.
- Loop forms (`loop`, `while`, `each`) MUST parse as expression-capable constructs where defined.

## Imports And Modules
- `draw` paths MUST parse segment-wise and preserve qualification.
- `ward` module declarations MAY nest.

## Macro Syntax
Declarative macro declarations and macro calls are part of v1.0 syntax surface. Expansion behavior
is specified by implementation and lowering phases.

## Conformance Notes
- A conforming parser MUST avoid infinite loops on malformed input.
- A conforming parser SHOULD preserve enough CST structure for formatter and IDE use-cases.
