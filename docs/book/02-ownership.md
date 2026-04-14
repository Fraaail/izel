# 2. Ownership

Ownership is the core safety model in Izel.

## Rules In One Page

1. A value has one owner.
2. Moving transfers ownership.
3. Borrowing creates temporary access without transfer.
4. Mutable access must be unique.

## Moves

```izel
forge consume(x: i32) -> i32 {
	x
}

forge demo() -> i32 {
	let a = 10
	let b = consume(a)
	b
}
```

## Borrows

Immutable borrow:

```izel
forge read_only(x: &i32) -> i32 {
	*x
}
```

Mutable borrow:

```izel
forge bump(x: &~i32) {
	*x = *x + 1
}
```

## Borrow Conflicts

The checker rejects overlapping alias patterns that could introduce races or stale reads.

Typical conflict:
- active immutable borrow
- attempted mutable borrow of same binding

## Non-Lexical Lifetime Behavior

Izel's borrow analysis is flow-sensitive. Once a borrow is no longer used, later borrows may be
accepted in the same lexical scope.

## Ownership And Zones

Zone-backed allocations are still owned values, but their valid lifetime cannot exceed zone scope.
Any reference escaping the zone boundary is rejected.

## Practical Advice

- Keep mutable borrows short.
- Prefer small helper forges for mutation-heavy paths.
- Pass immutable references by default.
- Move only when transfer of responsibility is the intent.

Next chapter: types and how ownership constraints interact with generic and user-defined forms.
