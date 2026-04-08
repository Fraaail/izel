# 1. Lexical Structure (Normative)

## Source Encoding
- Source files MUST be UTF-8.
- File extensions are `.iz` for source and `.izm` for module bundles.

## Line Endings
- Implementations MUST accept LF line endings.
- Implementations SHOULD accept CRLF line endings and normalize consistently in diagnostics.

## Comments
- Line comment: `// ...`.
- Block comment: `/~ ... ~/`.
- Doc comments are `///` and `//!`.

Comment trivia MAY be preserved in CST for formatting and tooling.

## Keywords And Sigils
- Language keywords and reserved words are defined in `docs/project_overview.md` section 4.
- Sigils such as `~`, `!`, `@`, and `|>` are lexical tokens with fixed meaning.

Keywords MUST NOT be accepted as plain identifiers in positions where an identifier is required,
unless explicitly introduced as contextual in a future edition.

## Literals
- Integer, float, string, bool, and nil literals are part of core lexical syntax.
- Numeric separators (`_`) are permitted where grammar allows.

Literal tokenization requirements:
- Invalid escape sequences MUST produce diagnostics or explicit error-token forms.
- Unterminated string/char literals MUST produce diagnostics and MUST NOT crash lexing.
- Lexing MUST make forward progress after malformed literals.

## Identifiers
- Identifiers MUST be valid Unicode identifier sequences.
- Keywords are not valid identifiers unless escaped by future language rules.

## Token Stream Guarantees
- The lexer MUST emit an EOF token.
- Token spans MUST be monotonic by source position.
- Trivia retention MAY be implementation-defined, but token order MUST reflect source order.

## Error Recovery
Lexical errors MUST be recoverable enough to allow parser continuation where possible.
