async function loadPlayground() {
  const output = document.getElementById("output");

  try {
    const wasm = await import("./pkg/izel_playground.js");
    await wasm.default();

    const runButton = document.getElementById("run");
    const source = document.getElementById("source");

    const run = async () => {
      const frontend = wasm.repl_eval(source.value);
      output.textContent = `Frontend:\n${frontend}`;

      if (!frontend.includes("Status: typecheck passed")) {
        return;
      }

      output.textContent += "\n\nRuntime:\nRunning native driver...";

      try {
        const response = await fetch("/api/run", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({ source: source.value }),
        });

        const data = await response.json();

        const runtimeOutput = [
          data.ok ? "Status: run passed" : "Status: run failed",
          data.status !== null && data.status !== undefined
            ? `Exit: ${data.status}`
            : "Exit: unavailable",
          "--- stdout ---",
          data.stdout || "",
          "--- stderr ---",
          data.stderr || "",
        ]
          .filter(Boolean)
          .join("\n")
          .trimEnd();

        output.textContent = `Frontend:\n${frontend}\n\nRuntime:\n${runtimeOutput}`;
      } catch (error) {
        output.textContent =
          `Frontend:\n${frontend}\n\nRuntime:\n` +
          "Status: run unavailable\n" +
          "Could not reach runtime endpoint /api/run.\n" +
          "Run npm run serve (runtime server), not static-only serve.\n\n" +
          String(error);
      }
    };

    runButton.addEventListener("click", run);
    source.addEventListener("keydown", (event) => {
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        run().catch((error) => {
          output.textContent = `run failed: ${String(error)}`;
        });
      }
    });

    output.textContent =
      "WASM playground loaded. Press Run or Cmd/Ctrl+Enter for frontend + runtime execution.";
  } catch (err) {
    output.textContent =
      "WASM module not built yet. Run npm run build:wasm from tools/playground first.\n\n" +
      String(err);
  }
}

loadPlayground();
