# 5. Memory And Ownership (Normative)

## Ownership
- Values have a single owner unless borrowed.
- Moves invalidate prior owners.

Use-after-move in non-copy contexts MUST be rejected.

## Borrowing
- Mutable and immutable borrow rules are enforced statically.
- Conflicting borrows MUST be rejected.

At any point in control flow, mutable aliasing MUST preserve uniqueness constraints.

## Lifetimes
- Region/lifetime constraints are inferred where possible.
- Escapes beyond valid region MUST produce diagnostics.

Non-lexical lifetime analysis MAY be implementation-specific, but safety outcomes MUST remain
sound under this model.

## Raw Escape Hatches
- `raw` is the explicit unsafe boundary.
- Safety guarantees outside `raw` remain enforced.

## Drop And Cleanup Semantics
Implementations MUST ensure deterministic cleanup behavior for owned values at scope end,
including path-sensitive control flow exits where required by the memory model.

## Region-Oriented Allocation
Zone interactions are specified in Chapter 8 and are considered part of memory safety rules.
