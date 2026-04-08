# 3. Types And Kinds (Normative)

## Primitive Types
v1.0 primitive kinds include integer, floating-point, bool, str, void, and pointer forms.

Primitive typing MUST be deterministic and architecture-stable except where pointer-size-dependent
types are explicitly used.

## Compound Types
Supported compound forms include:
- optionals,
- pointers,
- function types,
- user-defined `shape` and `scroll` types,
- witness types.

Implementations MAY add internal type representations, but external typing behavior MUST remain
consistent with this chapter.

## Inference And Unification
- Type inference is Hindley-Milner style with unification.
- Ambiguous or inconsistent constraints MUST produce diagnostics.

Inference engines MUST avoid accepting ill-typed programs due to fallback coercions not specified
by language rules.

## Coherence
- Weave coherence and orphan restrictions are enforced.
- Conflicting implementations MUST be rejected.

## Function Types
Function signatures include:
- parameter types,
- return type,
- effect obligations (see Chapter 4).

## Generic Constraints
Generic constraints MUST be checked at use sites and implementation sites.

## Associated Types
Associated types in weave declarations are part of type identity and MUST participate in
constraint checking.

## Optional And Never-Like Paths
Optional typing and divergence/non-returning control flow paths MUST unify according to
implementation's formal rules without violating soundness.
