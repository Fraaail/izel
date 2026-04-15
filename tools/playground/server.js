const http = require("http");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { spawnSync } = require("child_process");

const PORT = Number(process.env.PORT || 4173);
const ROOT_DIR = __dirname;
const REPO_ROOT = path.resolve(__dirname, "..", "..");
const MAX_BODY_BYTES = 1_000_000;

function contentTypeFor(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".css")) return "text/css; charset=utf-8";
  if (filePath.endsWith(".json")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".wasm")) return "application/wasm";
  if (filePath.endsWith(".map")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".ico")) return "image/x-icon";
  return "text/plain; charset=utf-8";
}

function sendJson(res, statusCode, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(statusCode, {
    "Content-Type": "application/json; charset=utf-8",
    "Content-Length": Buffer.byteLength(body),
  });
  res.end(body);
}

function readRequestBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;

    req.on("data", (chunk) => {
      size += chunk.length;
      if (size > MAX_BODY_BYTES) {
        reject(new Error("request body too large"));
        req.destroy();
        return;
      }
      chunks.push(chunk);
    });

    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

function runIzelSource(source) {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "izel-playground-"));
  const sourcePath = path.join(tempDir, "playground.iz");
  fs.writeFileSync(sourcePath, source, "utf8");

  try {
    const result = spawnSync(
      "bash",
      [
        "tools/ci/with_llvm_env.sh",
        "cargo",
        "run",
        "-p",
        "izel_driver",
        "--",
        "--run",
        sourcePath,
      ],
      {
        cwd: REPO_ROOT,
        encoding: "utf8",
        timeout: 120_000,
      }
    );

    return {
      ok: result.status === 0,
      status: result.status,
      signal: result.signal,
      stdout: result.stdout || "",
      stderr: result.stderr || "",
      error: result.error ? String(result.error.message || result.error) : null,
    };
  } finally {
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
}

async function handleRuntimeRequest(req, res) {
  let body;
  try {
    body = await readRequestBody(req);
  } catch (error) {
    sendJson(res, 413, { ok: false, error: String(error.message || error) });
    return;
  }

  let payload;
  try {
    payload = JSON.parse(body);
  } catch {
    sendJson(res, 400, { ok: false, error: "invalid JSON payload" });
    return;
  }

  if (!payload || typeof payload.source !== "string") {
    sendJson(res, 400, { ok: false, error: "payload.source must be a string" });
    return;
  }

  const result = runIzelSource(payload.source);
  sendJson(res, result.ok ? 200 : 500, result);
}

function safeResolveStaticPath(requestPath) {
  const rawPath = requestPath === "/" ? "/index.html" : requestPath;
  const normalized = path.normalize(decodeURIComponent(rawPath)).replace(/^([.][.][/\\])+/, "");
  const absolute = path.resolve(ROOT_DIR, `.${normalized}`);

  if (!absolute.startsWith(ROOT_DIR)) {
    return null;
  }

  if (fs.existsSync(absolute) && fs.statSync(absolute).isFile()) {
    return absolute;
  }

  return null;
}

function serveStatic(req, res) {
  const filePath = safeResolveStaticPath(new URL(req.url, "http://localhost").pathname);
  if (!filePath) {
    res.writeHead(404, { "Content-Type": "text/plain; charset=utf-8" });
    res.end("Not Found");
    return;
  }

  const data = fs.readFileSync(filePath);
  res.writeHead(200, {
    "Content-Type": contentTypeFor(filePath),
    "Content-Length": data.length,
  });
  res.end(data);
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, "http://localhost");

  if (req.method === "POST" && url.pathname === "/api/run") {
    await handleRuntimeRequest(req, res);
    return;
  }

  if (req.method === "GET" || req.method === "HEAD") {
    serveStatic(req, res);
    return;
  }

  res.writeHead(405, { "Content-Type": "text/plain; charset=utf-8" });
  res.end("Method Not Allowed");
});

server.listen(PORT, () => {
  console.log(`Izel playground server running on http://localhost:${PORT}`);
  console.log("Runtime endpoint: POST /api/run");
});
