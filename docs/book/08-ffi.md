# 8. FFI And Raw Blocks

Interfacing with external code requires explicit unsafe boundaries in Izel.

## Raw Blocks

`raw` marks operations outside normal safety guarantees.

Use raw for:
- pointer arithmetic,
- unchecked memory operations,
- low-level interoperability details.

Always document safety assumptions near raw code.

## Bridge Declarations

`bridge` blocks declare foreign functions and statics.

Supported ABI forms are validated by the type checker.

Bridge declarations are restricted to declaration-only forms; invalid bodies or unsupported ABI
usage are rejected.

## Inline Assembly

Inline asm is represented through constrained raw usage and validated templates.

Key constraints:
- must appear in raw context,
- must use valid template forms.

## Practical FFI Pattern

1. Keep bridge surface small.
2. Wrap raw calls in safe forge APIs.
3. Expose only validated domain types outward.

## Auditing Checklist

- Does each raw block state invariants?
- Are lifetimes and ownership transfers explicit?
- Are bridge signatures minimal and stable?

You now have the full introductory tour of Izel's core language and runtime model.
