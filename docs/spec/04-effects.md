# 4. Effect System (Normative)

## Effect Declarations
- `forge` signatures MAY declare effects (for example `!io`).
- Pure functions are effect-free unless explicitly declared otherwise.

An implementation MUST treat effect annotations as part of callable interface constraints.

## Propagation
- Calls propagate effect obligations to callers.
- Missing effect propagation is a type error.

Propagation MUST be transitive through call chains unless effects are explicitly contained by
validated boundaries.

## Boundaries
- Effect boundaries MAY mask contained effects when declared.
- Boundary usage is validated at type-check time.

Invalid boundary declarations MUST produce diagnostics.

## Conformance
Implementations MUST reject effect-unsafe calls and MUST preserve sound effect inference.

## Effect Sets
Effect sets are order-insensitive semantically, even if stored in ordered data structures.

## Testing Implications
Effect compatibility between weave declarations and implementations MUST be checked where effect
sets are part of method obligations.
