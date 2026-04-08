# 6. Witness Types (Normative)

## Model
Witness values encode compile-time proof obligations as types.

Witnesses are treated as evidence-carrying values for type-level predicates.

## Built-Ins
v1.0 includes built-in witnesses such as `NonZero`, `InBounds`, and `Sorted`.

## Construction Rules
- Witness construction is restricted to proof-valid paths or explicit `raw` contexts.
- Invalid witness construction MUST be rejected.

Implementations MUST prevent accidental witness fabrication in safe contexts.

## Runtime Elision
Where witness evidence is present, redundant runtime assertions MAY be omitted.

## Interop With Type Checking
Witness-bearing types MUST participate in normal unification and call checking.

## Built-In Utility Surfaces
Where built-in witnesses provide helper constructors or assertions, those APIs MUST preserve
witness invariants and signal violations consistently.
