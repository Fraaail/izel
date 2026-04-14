# VS Code Izel Language Support

This extension package provides syntax highlighting and language configuration for `.iz` files.

## Local Usage

1. Open this folder in VS Code:
   - `tools/grammar/vscode-izel`
2. Press `F5` to launch an Extension Development Host.
3. Open any `.iz` file to verify highlighting.

## Package as VSIX

```bash
cd tools/grammar/vscode-izel
npm install
npm run package
```

The produced `.vsix` can be installed with:

```bash
code --install-extension <generated-file>.vsix
```
