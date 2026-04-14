# 6. Memory Zones

Zones provide deterministic region-based memory management.

## Why Zones

Use zones when you want:
- many short-lived allocations,
- predictable teardown,
- no per-object free calls.

## Basic Form

```izel
zone request {
	~sum = 0
	each i in 0..10 {
		~sum = sum + i
	}
}
```

## Allocation Accessors

Within zone scope, allocator accessors are valid:
- `zone::allocator()`
- `<zone_name>::allocator()`

Outside zone scope, these are rejected.

## Escape Safety

References into zone-owned data cannot outlive the zone. The checker enforces this structurally.

## Nested Zones

Nested zones are legal and useful for hierarchical lifetimes, such as request-level and
render-pass-level memory.

## Guidance

- Prefer zones for temporary batch transforms.
- Keep cross-zone references explicit and short.
- Use normal ownership for long-lived structures.

Next chapter: concurrency and async surfaces.
