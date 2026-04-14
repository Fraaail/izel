use izel_ast_lower::Lowerer;
use izel_lexer::{Lexer, TokenKind};
use izel_parser::ast;
use izel_parser::Parser;
use izel_span::SourceId;

fn lower_module(source: &str) -> ast::Module {
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
    let cst = parser.parse_source_file();
    Lowerer::new(source).lower_module(&cst)
}

#[test]
fn lowers_shape_and_forge_items() {
    let source = r#"
shape Point { x: i32, y: i32 }
forge main() { give 0 }
"#;
    let module = lower_module(source);

    assert!(module
        .items
        .iter()
        .any(|item| { matches!(item, ast::Item::Shape(shape) if shape.name == "Point") }));
    assert!(module
        .items
        .iter()
        .any(|item| { matches!(item, ast::Item::Forge(forge) if forge.name == "main") }));
}

#[test]
fn lowers_draw_path_segments() {
    let module = lower_module("draw std::io");
    let draw = module
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Draw(draw) => Some(draw),
            _ => None,
        })
        .expect("expected one draw item");

    assert_eq!(draw.path, vec!["std".to_string(), "io".to_string()]);
    assert!(!draw.is_wildcard);
}

#[test]
fn lowers_zone_allocator_call_shape() {
    let module = lower_module(
        r#"
forge main() {
    zone arena {
        let alloc = zone::allocator()
    }
}
"#,
    );

    let forge = module
        .items
        .iter()
        .find_map(|item| match item {
            ast::Item::Forge(forge) if forge.name == "main" => Some(forge),
            _ => None,
        })
        .expect("expected forge main");

    let body = forge.body.as_ref().expect("main must have a body");
    let zone_body = body
        .stmts
        .iter()
        .find_map(|stmt| match stmt {
            ast::Stmt::Expr(ast::Expr::Zone { body, .. }) => Some(body),
            _ => None,
        })
        .or_else(|| {
            body.expr.as_deref().and_then(|expr| match expr {
                ast::Expr::Zone { body, .. } => Some(body),
                _ => None,
            })
        })
        .expect("expected zone expression in main body");

    let init_expr = zone_body
        .stmts
        .iter()
        .find_map(|stmt| match stmt {
            ast::Stmt::Let {
                init: Some(expr), ..
            } => Some(expr),
            _ => None,
        })
        .expect("expected let initializer inside zone body");

    match init_expr {
        ast::Expr::Call(callee, args) => {
            assert!(args.is_empty(), "zone allocator call should have no args");
            match callee.as_ref() {
                ast::Expr::Path(path, _) => {
                    assert_eq!(
                        path,
                        &vec!["zone".to_string(), "allocator".to_string()],
                        "zone allocator should lower as a two-segment path"
                    );
                }
                ast::Expr::Member(obj, method, _) => {
                    assert_eq!(method, "allocator");
                    assert!(matches!(obj.as_ref(), ast::Expr::Ident(name, _) if name == "zone"));
                }
                other => panic!("unexpected call callee for zone allocator: {other:?}"),
            }
        }
        other => panic!("zone allocator let initializer should lower to Call, got: {other:?}"),
    }
}
