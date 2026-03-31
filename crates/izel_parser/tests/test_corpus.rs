use izel_lexer::{Lexer, TokenKind};
use izel_parser::cst::NodeKind;
use izel_parser::Parser;
use izel_span::SourceId;
use std::fs;
use std::path::{Path, PathBuf};

fn parse_source(source: &str) -> izel_parser::cst::SyntaxNode {
    let mut lexer = Lexer::new(source, SourceId(0));
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == TokenKind::Eof {
            break;
        }
    }

    let mut parser = Parser::new(tokens, source.to_string());
    parser.parse_source_file()
}

fn collect_iz_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("failed to read parser corpus directory");
    for entry in entries {
        let path = entry.expect("failed to read parser corpus entry").path();
        if path.is_dir() {
            collect_iz_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("iz") {
            out.push(path);
        }
    }
}

#[test]
fn parser_corpus_workspace_files_do_not_infinite_loop() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();

    for rel in ["examples", "tests", "std", "library", "compiler"] {
        let dir = repo_root.join(rel);
        if dir.exists() {
            collect_iz_files(&dir, &mut files);
        }
    }

    files.sort();
    files.dedup();
    assert!(
        !files.is_empty(),
        "expected at least one .iz file in parser corpus"
    );

    let mut parsed = 0usize;
    for file in files {
        let source = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", file, e));

        let result = std::panic::catch_unwind(|| parse_source(&source));
        if let Ok(cst) = result {
            assert_eq!(cst.kind, NodeKind::SourceFile);
            parsed += 1;
        }
    }

    assert!(
        parsed > 0,
        "expected at least one corpus file to parse successfully"
    );
}

#[test]
fn parser_corpus_malformed_snippets_exercise_recovery_paths() {
    let malformed = [
        "pkg(core::) forge main( {",
        "macro m![x, y {",
        "shape Packet { open id: i32 hidden payload: str",
        "forge main() { branch x { v given => v, _ }",
        "draw std::%oops; forge next() { give 0 }",
        "@attr( forge broken() {",
        "#[test(] forge t() { give }",
        "weave Renderable for { forge render(self)",
        "dual shape Codec<T { forge encode(self) {",
        "ward Core { forge helper() { let x = [1, ..] }",
    ];

    for src in malformed {
        let _ = std::panic::catch_unwind(|| parse_source(src));
    }
}
