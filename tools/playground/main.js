async function loadPlayground() {
  const output = document.getElementById("output");

  try {
    const wasm = await import("./pkg/izel_playground.js");
    await wasm.default();

    const runButton = document.getElementById("run");
    const source = document.getElementById("source");

    const run = () => {
      const result = wasm.repl_eval(source.value);
      output.textContent = result;
    };

    runButton.addEventListener("click", run);
    source.addEventListener("keydown", (event) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        run();
      }
    });

    output.textContent =
      "WASM playground loaded. Press Run or Cmd/Ctrl+Enter.";
  } catch (err) {
    output.textContent =
      "WASM module not built yet. Run npm run build:wasm from tools/playground first.\n\n" +
      String(err);
  }
}

loadPlayground();
