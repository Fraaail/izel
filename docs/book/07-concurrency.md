# 7. Concurrency

Izel supports both thread-based and async-style concurrency models.

## Threads And Channels

Thread-style coordination is represented through standard library concurrency wards.

Typical model:
- spawn work,
- communicate through channels,
- join at synchronization boundaries.

## Flow And Tide

`flow` marks asynchronous forge declarations.

`tide` awaits async results and composes multiple concurrent operations.

## Atomic And Shared State

Atomic primitives and synchronization constructs are exposed through standard library surfaces.

When sharing mutable state:
- prefer message passing first,
- then choose lock/atomic strategies deliberately.

## Effect Interaction

Concurrency often intersects with effects such as:
- `!thread`
- `!io`
- `!net`

Keep signatures explicit and transitive obligations clear.

## Debugging Tips

- Start with deterministic tests around small task units.
- Add synchronization only where races are real, not speculative.
- Keep mutable shared state minimal.

Next chapter: FFI and raw boundaries.
