# 5. Witness Types

Witnesses encode proof obligations in types.

## Mental Model

A witness says: this value satisfies a predicate, and the type system may rely on that fact.

## Built-In Witnesses

- `NonZero<T>`
- `InBounds<T>`
- `Sorted<T>`

These allow APIs to remove repeated runtime checks once evidence exists.

## Example: NonZero

```izel
forge divide(a: i32, b: NonZero<i32>) -> i32 {
	a
}
```

The divisor constraint is expressed in the type, not re-checked at each call site.

## Custom Witnesses

You can model domain proofs with unit predicate shapes.

```izel
shape IsPositive
```

Then expose controlled constructors through proof-valid paths.

## Construction Safety

Witness creation is gated:
- allowed in proof-valid contexts,
- allowed in explicit raw contexts,
- rejected elsewhere.

## Practical Guidance

- Prefer witness-carrying APIs at boundaries where invalid input is expensive.
- Keep proof-producing code small and auditable.
- Avoid raw witness construction unless absolutely required.

Next chapter: memory zones.
