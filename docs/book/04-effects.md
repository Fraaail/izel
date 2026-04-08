# 4. Effects

Effects describe what a forge can do besides returning a value.

## Why Effects Matter

Without effects, signatures hide behavior such as I/O or allocation. Izel makes these obligations
explicit and checked.

## Declaring Effects

```izel
forge read_config(path: str) -> str !io {
	path
}
```

## Propagation

If forge A calls forge B and B requires `!io`, then A must also declare compatible effects unless
the call is inside an approved boundary.

## Common Built-In Effects

- `!io`
- `!net`
- `!alloc`
- `!panic`
- `!unsafe`
- `!ffi`
- `!thread`

## Boundaries

Effect boundaries can contain effects and expose a narrower surface to callers.

Use boundaries when you intentionally encapsulate side effects behind a stable interface.

## Testing With Effects

Because effects are typed obligations, test doubles can be represented as regular weave
implementations with reduced effect sets where valid.

## Debugging Effect Errors

When you get an effect mismatch:
1. Start at the called forge's signature.
2. Follow transitive call edges upward.
3. Confirm whether a boundary should be introduced or whether propagation is correct.

Next chapter: witness types for proof-carrying values.
