# 7. Temporal Contracts (Normative)

## Preconditions And Postconditions
- `@requires` expresses preconditions.
- `@ensures` expresses postconditions.

Contract expressions MUST typecheck under the same typing rules as other boolean conditions.

## Compile-Time Evaluation
- Contracts over compile-time-known values are validated statically.
- Violations MUST emit diagnostics.

## Runtime Instrumentation
- For dynamic values, implementations MAY emit runtime assertions.
- Contract instrumentation behavior is controlled by compiler flags.

When runtime instrumentation is enabled, contract failures MUST be observable as explicit
assertion-like failures.

## Invariants
`#[invariant]` constraints on stateful structures are part of the v1.0 contract model.

Implementations SHOULD validate invariant preservation across methods that mutate structure state,
subject to available analysis precision.
