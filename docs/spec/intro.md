# Izel Language Specification v1.0

## Status
This document set is normative for Izel v1.0.

Unless explicitly marked as an example or note, all statements in files under `docs/spec/` are normative.

## Scope
The specification defines:
- Lexical rules and tokenization.
- Syntax and parsing structure.
- Type and effect semantics.
- Ownership, witnesses, contracts, zones, and duality semantics.
- Core language-level API surface references.

## Non-Goals
This specification does not guarantee:
- optimization strategy internals,
- backend-specific IR formatting,
- diagnostic wording stability.

Those concerns are implementation-defined unless explicitly stated in a normative chapter.

## Conformance
An implementation conforms to Izel v1.0 if it:
- Accepts programs valid under these rules.
- Rejects programs that violate these rules with diagnostics.
- Preserves the safety semantics of effects, ownership, witnesses, contracts, and zones.

## Normative Language
The key words MUST, MUST NOT, SHOULD, SHOULD NOT, and MAY are to be interpreted as normative
requirements.

## Compatibility Model
- Minor revisions MAY add diagnostics or stricter checks where behavior was previously unspecified.
- Revisions MUST NOT silently change accepted meaning of existing well-typed programs without
	explicit migration notes.

## Priority Of Documents
1. Files in `docs/spec/`.
2. Clarifying examples in `docs/book/`.
3. Project overview summaries in `docs/project_overview.md`.

Where summaries conflict with this specification, this specification wins.
