use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("failed to resolve repository root")
}

fn temp_iz_file(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    path.push(format!(
        "izel-driver-runtime-io-{}-{}.iz",
        std::process::id(),
        nonce
    ));
    fs::write(&path, content).expect("failed to write runtime io fixture");
    path
}

fn run_izelc_from_repo(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izelc"))
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to execute izelc")
}

fn extract_runtime_stdout(full_stdout: &str) -> String {
    let mut in_runtime = false;
    let mut between_markers = Vec::new();
    let mut after_footer = Vec::new();
    let mut seen_footer = false;

    for line in full_stdout.lines() {
        if line == "--- JIT Execution ---" {
            in_runtime = true;
            continue;
        }

        if !in_runtime && !seen_footer {
            continue;
        }

        if line.starts_with("JIT Exit Code:") {
            continue;
        }

        if line == "----------------------" {
            seen_footer = true;
            in_runtime = false;
            continue;
        }

        if in_runtime {
            between_markers.push(line.to_string());
        } else if seen_footer {
            after_footer.push(line.to_string());
        }
    }

    let lines = if between_markers.is_empty() {
        after_footer
    } else {
        between_markers
    };

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[test]
fn runtime_io_streams_snapshot_stdout_and_stderr_separately() {
    let source = r#"draw std/io

forge main() -> int {
    println("stdout-line")
    eprintln("stderr-line")
    println_int(7)
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    let stdout_snapshot = "stdout-line\n7\n";
    let stderr_snapshot = "stderr-line\n";

    assert_eq!(runtime_stdout, stdout_snapshot);
    assert_eq!(stderr, stderr_snapshot);
}

#[test]
fn runtime_io_streams_preserve_escaped_string_snapshots() {
    let source = r#"draw std/io

forge main() -> int {
    println("stdout-\x41\tend")
    eprintln("stderr-\u{1F600}")
    give 0
}
"#;

    let input = temp_iz_file(source);
    let input_arg = input.to_string_lossy().to_string();

    let output = run_izelc_from_repo(&["--run", &input_arg]);
    let _ = fs::remove_file(&input);

    assert!(
        output.status.success(),
        "compile+run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let runtime_stdout = extract_runtime_stdout(&stdout);
    let stdout_snapshot = "stdout-A\tend\n";
    let stderr_snapshot = "stderr-😀\n";

    assert_eq!(runtime_stdout, stdout_snapshot);
    assert_eq!(stderr, stderr_snapshot);
}
