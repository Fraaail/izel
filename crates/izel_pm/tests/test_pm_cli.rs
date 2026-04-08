use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn run_izel_pm(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_izel"))
        .args(args)
        .output()
        .expect("failed to execute izel_pm")
}

fn unique_temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after UNIX epoch")
        .as_nanos();
    path.push(format!(
        "izel-pm-cli-test-{}-{}-{}",
        std::process::id(),
        label,
        nonce
    ));
    path
}

#[test]
fn cli_help_and_build_paths_are_callable() {
    let help = run_izel_pm(&[]);
    assert!(
        help.status.success(),
        "help command failed: {}",
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(String::from_utf8_lossy(&help.stdout).contains("izel new <name>"));

    let build = run_izel_pm(&["build"]);
    assert!(
        build.status.success(),
        "build command failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(String::from_utf8_lossy(&build.stdout).contains("Build command accepted"));
}

#[test]
fn cli_run_paths_cover_empty_and_forwarded_args() {
    let run_empty = run_izel_pm(&["run"]);
    assert!(
        run_empty.status.success(),
        "run command failed: {}",
        String::from_utf8_lossy(&run_empty.stderr)
    );
    assert!(String::from_utf8_lossy(&run_empty.stdout).contains("Run command accepted."));

    let run_args = run_izel_pm(&["run", "--", "alpha", "beta"]);
    assert!(
        run_args.status.success(),
        "run with args failed: {}",
        String::from_utf8_lossy(&run_args.stderr)
    );
    assert!(String::from_utf8_lossy(&run_args.stdout).contains("Run command accepted with args"));
}

#[test]
fn cli_new_command_creates_project_files() {
    let root = unique_temp_path("new-success");
    let root_arg = root.to_string_lossy().to_string();

    let output = run_izel_pm(&["new", &root_arg]);

    assert!(
        output.status.success(),
        "new command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Created Izel project"));

    assert!(root.join("Izel.toml").exists());
    assert!(root.join("src/main.iz").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn cli_new_command_reports_creation_errors() {
    let output = run_izel_pm(&["new", "/dev/null/izel-pm-nope"]);

    assert!(
        !output.status.success(),
        "new should fail when target root is invalid"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to create project"));
}

#[test]
fn cli_unknown_command_returns_error_and_usage() {
    let output = run_izel_pm(&["deploy"]);

    assert!(!output.status.success(), "unknown command should fail");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown command: deploy"));
    assert!(stderr.contains("izel run [-- <args>]"));
}

#[test]
fn cli_all_subcommand_success_paths_are_callable() {
    let root = unique_temp_path("new-kinds");
    let root_lib = unique_temp_path("new-lib");
    let root_ws = unique_temp_path("new-workspace");
    let root_arg = root.to_string_lossy().to_string();
    let root_lib_arg = root_lib.to_string_lossy().to_string();
    let root_ws_arg = root_ws.to_string_lossy().to_string();

    let cases: Vec<(Vec<&str>, &str)> = vec![
        (
            vec!["build", "--release", "--target", "wasm32-unknown-unknown"],
            "Build command accepted",
        ),
        (vec!["test"], "Test command accepted"),
        (
            vec!["test", "lint", "--threads", "2"],
            "Test command accepted",
        ),
        (vec!["bench", "pipeline"], "Bench command accepted"),
        (vec!["check"], "Check command accepted"),
        (vec!["fmt"], "Fmt command accepted"),
        (vec!["fmt", "--check"], "Fmt command accepted"),
        (vec!["lint"], "Lint command accepted"),
        (vec!["doc"], "Doc command accepted"),
        (vec!["doc", "--open"], "Doc command accepted"),
        (vec!["add", "demo"], "Add command accepted"),
        (vec!["add", "demo@1.2.3", "--dev"], "Add command accepted"),
        (vec!["remove", "demo"], "Remove command accepted"),
        (vec!["update"], "Update command accepted"),
        (vec!["publish"], "Publish command accepted"),
        (vec!["clean"], "Clean command accepted"),
        (vec!["tree"], "Tree command accepted"),
        (vec!["audit"], "Audit command accepted"),
        (vec!["new", &root_arg, "--bin"], "Created Izel project"),
        (vec!["new", &root_lib_arg, "--lib"], "Created Izel project"),
        (
            vec!["new", &root_ws_arg, "--workspace"],
            "Created Izel project",
        ),
    ];

    for (args, needle) in cases {
        let output = run_izel_pm(&args);
        assert!(
            output.status.success(),
            "command {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(needle),
            "command {:?} missing output marker '{}'",
            args,
            needle
        );
    }

    assert!(root.join("src/main.iz").exists());
    assert!(root_lib.join("src/lib.iz").exists());
    assert!(root_ws.join("Izel.toml").exists());

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&root_lib);
    let _ = fs::remove_dir_all(&root_ws);
}

#[test]
fn cli_usage_errors_cover_invalid_forms() {
    let cases: Vec<Vec<&str>> = vec![
        vec!["bench", "a", "b"],
        vec!["check", "x"],
        vec!["fmt", "--bad"],
        vec!["lint", "x"],
        vec!["doc", "--bad"],
        vec!["remove"],
        vec!["update", "x"],
        vec!["publish", "x"],
        vec!["clean", "x"],
        vec!["tree", "x"],
        vec!["audit", "x"],
        vec!["build", "--target"],
        vec!["build", "--bad"],
        vec!["test", "--threads"],
        vec!["test", "--threads", "0"],
        vec!["test", "--threads", "abc"],
        vec!["test", "--bogus"],
        vec!["add"],
        vec!["add", "@"],
        vec!["add", "demo", "--bad"],
        vec!["new", "pkg", "--bad"],
        vec!["new", "pkg", "--lib", "--workspace"],
    ];

    for args in cases {
        let output = run_izel_pm(&args);
        assert!(
            !output.status.success(),
            "invalid command {:?} unexpectedly succeeded",
            args
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("usage:")
                || stderr.contains("unknown command")
                || stderr.contains("choose only one")
                || stderr.contains("positive integer")
                || stderr.contains("package spec must"),
            "invalid command {:?} missing usage/error output: {}",
            args,
            stderr
        );
    }
}
