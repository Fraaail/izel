# tree-sitter-izel

Tree-sitter grammar for Izel syntax highlighting and editor integration.

## Scope

This grammar targets the core declaration and statement surface used by the compiler roadmap:

- `forge`, `shape`, `scroll`, `ward`, `draw`
- blocks, conditionals (`given`/`else`), and `while`
- path expressions (`std::io::println`) and calls
- core literals and comments

## Usage

```bash
cd tools/grammar/tree-sitter-izel
npm install
npm run generate
npm run test
```

## Notes

The grammar focuses on stable core syntax used by compiler tests and examples.
