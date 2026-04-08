# The Izel Book

Welcome to Izel, a systems programming language designed around explicitness, safety, and
predictability.

This book is practical first:
- You will write and run code quickly.
- You will learn why the language makes specific trade-offs.
- You will build intuition for ownership, effects, witnesses, contracts, and zones.

## What Izel Optimizes For

Izel treats these as first-class constraints:
- No hidden runtime behavior.
- No implicit side effects.
- No garbage collector.
- Strong static diagnostics before execution.

In practice this means signatures are rich, control flow is explicit, and advanced guarantees
such as effect-tracking and witness proofs are part of day-to-day programming.

## Quick Taste

```izel
forge add(a: i32, b: i32) -> i32 {
	a + b
}

forge main() -> i32 {
	add(20, 22)
}
```

## Book Structure

1. Getting Started: install, build, run.
2. Ownership: moves, borrows, and why aliasing rules matter.
3. Types: shape, scroll, weave, generics, and contracts with the type system.
4. Effects: how signatures describe interactions with the world.
5. Witnesses: proof-carrying values that remove runtime checks.
6. Zones: deterministic region-based memory management.
7. Concurrency: thread and async surfaces.
8. FFI: raw blocks, bridge declarations, and unsafe boundaries.

## Reading Guidance

- Read chapters in order if you are new to Izel.
- Jump to a topic chapter if you already know the core syntax.
- Keep the spec open for normative rules when details matter.

Specification companion:
- `docs/spec/intro.md`

Let's begin with the toolchain and your first executable.
