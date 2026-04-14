# 3. Types

Izel combines strict static typing with practical inference.

## Primitive Types

- Integers: signed and unsigned fixed widths, plus pointer-sized forms.
- Floats: `f32`, `f64`.
- Boolean and character.
- String slices and pointer forms.

## Composite Types

- Tuples
- Arrays and slices
- Optional values (`?T`)
- Function types
- User-defined shapes and scrolls

## Shape

```izel
shape Point {
	x: f64,
	y: f64,
}
```

## Scroll

```izel
scroll ResultLike<T, E> {
	Ok(T),
	Err(E),
}
```

## Weave

```izel
weave Printable {
	forge print(&self)
}
```

## Generics

```izel
forge identity<T>(x: T) -> T {
	x
}
```

Inference handles most local cases, while explicit annotations remain useful at module boundaries.

## Associated Types And Bounds

Weave bounds and associated types allow precise constraints without dynamic overhead.

## Type Errors

Type diagnostics in Izel should answer:
- what was expected,
- what was inferred,
- where the mismatch originated.

If you see a deep mismatch, inspect earlier inferred bindings first; the first bad assumption often
appears before the final error site.

Next chapter: effects, where behavior-level contracts join the type signature.
