use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct PipelineReport {
    token_count: usize,
    diagnostics: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests crate should live under repo/tests")
        .to_path_buf()
}

fn run_frontend_pipeline(path: &Path) -> Result<PipelineReport> {
    let source = fs::read_to_string(path)?;

    let source_id = izel_span::SourceId(0);
    let mut lexer = izel_lexer::Lexer::new(&source, source_id);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == izel_lexer::TokenKind::Eof {
            break;
        }
    }

    let mut parser = izel_parser::Parser::new(tokens.clone(), source.clone());
    let cst = parser.parse_source_file();

    let base_path = path.parent().map(|p| p.to_path_buf());
    let mut resolver = izel_resolve::Resolver::new(base_path);
    resolver.resolve_source_file(&cst, &source);

    let ast_lowerer = izel_ast_lower::Lowerer::new(&source);
    let ast = ast_lowerer.lower_module(&cst);

    let mut typeck = izel_typeck::TypeChecker::with_builtins();
    typeck.span_to_def = resolver.def_ids.clone();

    let mut ast_modules = HashMap::new();
    let loaded_csts = resolver
        .loaded_csts
        .read()
        .expect("loaded_csts lock poisoned");
    for (name, (loaded_cst, loaded_source)) in loaded_csts.iter() {
        let lowerer = izel_ast_lower::Lowerer::new(loaded_source);
        ast_modules.insert(name.clone(), lowerer.lower_module(loaded_cst));
    }
    drop(loaded_csts);

    typeck.check_project(&ast, ast_modules);

    let diagnostics = typeck
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect::<Vec<_>>();

    Ok(PipelineReport {
        token_count: tokens.len(),
        diagnostics,
    })
}

fn render_suite_summary() -> Result<String> {
    let root = repo_root();
    let pass_cases = vec![
        (
            "driver_custom_iterator",
            root.join("crates/izel_driver/tests/fixtures/custom_iterator.iz"),
        ),
        (
            "driver_custom_witness",
            root.join("crates/izel_driver/tests/fixtures/custom_witness.iz"),
        ),
        (
            "contracts_pass_requires",
            root.join("tests/contracts/pass_requires.iz"),
        ),
        (
            "effects_pass_declared_io",
            root.join("tests/effects/pass_declared_io.iz"),
        ),
        (
            "witnesses_pass_proof_construction",
            root.join("tests/witnesses/pass_proof_construction.iz"),
        ),
        (
            "zones_pass_zone_allocator",
            root.join("tests/zones/pass_zone_allocator.iz"),
        ),
    ];
    let fail_cases = vec![
        (
            "effects_violation",
            root.join("tests/compile_fail/effects_violation.iz"),
        ),
        (
            "contracts_fail_requires",
            root.join("tests/contracts/fail_requires.iz"),
        ),
        (
            "effects_fail_pure_boundary",
            root.join("tests/effects/fail_pure_boundary.iz"),
        ),
        (
            "witnesses_fail_direct_witness",
            root.join("tests/witnesses/fail_direct_witness.iz"),
        ),
        (
            "zones_fail_zone_allocator_scope",
            root.join("tests/zones/fail_zone_allocator_scope.iz"),
        ),
    ];

    let mut out = String::new();
    out.push_str("# Integration Snapshot\n");

    for (name, path) in pass_cases {
        let report = run_frontend_pipeline(&path)?;
        let status = if report.diagnostics.is_empty() {
            "ok"
        } else {
            "diag"
        };
        out.push_str(&format!(
            "PASS {} tokens={} diagnostics={} status={}\n",
            name,
            report.token_count,
            report.diagnostics.len(),
            status
        ));
    }

    for (name, path) in fail_cases {
        let report = run_frontend_pipeline(&path)?;
        let status = if report.diagnostics.is_empty() {
            "ok"
        } else {
            "diag"
        };
        let first = report
            .diagnostics
            .first()
            .map(String::as_str)
            .unwrap_or("<none>");
        out.push_str(&format!(
            "FAIL {} tokens={} diagnostics={} status={} first_diag={}\n",
            name,
            report.token_count,
            report.diagnostics.len(),
            status,
            first
        ));
    }

    Ok(out)
}

#[test]
fn test_front_end_pipeline() -> Result<()> {
    let root = repo_root();
    let path = root.join("crates/izel_driver/tests/fixtures/custom_iterator.iz");
    let report = run_frontend_pipeline(&path)?;
    assert!(
        report.diagnostics.is_empty(),
        "expected clean pass fixture diagnostics: {:?}",
        report.diagnostics
    );
    Ok(())
}

#[test]
fn test_static_analysis() -> Result<()> {
    let root = repo_root();
    let path = root.join("tests/compile_fail/effects_violation.iz");
    let report = run_frontend_pipeline(&path)?;
    assert!(
        !report.diagnostics.is_empty(),
        "expected diagnostics for compile_fail fixture"
    );
    Ok(())
}

#[test]
fn test_unique_features() -> Result<()> {
    let root = repo_root();
    let path = root.join("crates/izel_driver/tests/fixtures/custom_witness.iz");
    let report = run_frontend_pipeline(&path)?;
    assert!(
        report.diagnostics.is_empty(),
        "expected witness fixture to pass, diagnostics: {:?}",
        report.diagnostics
    );
    Ok(())
}

#[test]
fn test_snapshot_integration_suite() -> Result<()> {
    let root = repo_root();
    let snapshot_path = root.join("tests/snapshots/integration_suite.snap");
    let expected = fs::read_to_string(snapshot_path)?;
    let actual = render_suite_summary()?;
    assert_eq!(actual, expected, "integration snapshot mismatch");
    Ok(())
}

#[test]
fn test_category_fixture_directories_are_populated() -> Result<()> {
    let root = repo_root();
    let iz_fixture_dirs = [
        root.join("tests/contracts"),
        root.join("tests/effects"),
        root.join("tests/witnesses"),
        root.join("tests/zones"),
        root.join("tests/compile_pass"),
        root.join("tests/run_pass"),
        root.join("tests/run_fail"),
    ];

    for dir in iz_fixture_dirs {
        let iz_count = fs::read_dir(&dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "iz"))
            .count();

        assert!(
            iz_count > 0,
            "expected at least one .iz fixture in {:?}",
            dir
        );
    }

    let snapshot = root.join("tests/snapshots/integration_suite.snap");
    assert!(
        snapshot.exists(),
        "expected snapshot file at {:?}",
        snapshot
    );

    Ok(())
}
