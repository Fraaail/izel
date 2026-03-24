use izel_ast_lower::Lowerer;
use izel_hir::lower::HirLowerer;
use izel_hir::HirItem;
use izel_lexer::{Lexer, TokenKind};
use izel_parser::Parser;

fn tokenize(source: &str) -> Vec<izel_lexer::Token> {
    let mut lexer = Lexer::new(source, izel_span::SourceId(0));
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let is_eof = token.kind == TokenKind::Eof;
        tokens.push(token);
        if is_eof {
            break;
        }
    }

    tokens
}

#[test]
fn test_lower_module_retains_echo_items() {
    let source = r#"
		echo {
			let x = 1 + 2
			x
		}

		forge main() {
			give
		}
	"#;

    let tokens = tokenize(source);
    let mut parser = Parser::new(tokens, source.to_string());
    parser.source = source.to_string();
    let cst = parser.parse_source_file();

    let mut resolver = izel_resolve::Resolver::new(None);
    resolver.resolve_source_file(&cst, source);

    let ast_lowerer = Lowerer::new(source);
    let ast = ast_lowerer.lower_module(&cst);

    let typeck = izel_typeck::TypeChecker::new();
    let hir_lowerer = HirLowerer::new(&resolver, &typeck.def_types);
    let hir = hir_lowerer.lower_module(&ast);

    assert!(
        hir.items
            .iter()
            .any(|item| matches!(item, HirItem::Echo(_))),
        "echo item should be preserved in HIR"
    );
}
