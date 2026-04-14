use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn repl_eval(source: &str) -> String {
    if source.trim().is_empty() {
        return "error: source is empty".to_string();
    }

    let source_id = izel_span::SourceId(1);
    let mut lexer = izel_lexer::Lexer::new(source, source_id);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let kind = token.kind;
        tokens.push(token);
        if kind == izel_lexer::TokenKind::Eof {
            break;
        }
    }

    let mut parser = izel_parser::Parser::new(tokens.clone(), source.to_string());
    let cst = parser.parse_source_file();

    let lowerer = izel_ast_lower::Lowerer::new(source);
    let ast = lowerer.lower_module(&cst);

    let mut typeck = izel_typeck::TypeChecker::with_builtins();
    typeck.check_ast(&ast);

    let mut out = String::new();
    out.push_str(&format!("Tokens: {}\n", tokens.len()));

    if typeck.diagnostics.is_empty() {
        out.push_str("Status: typecheck passed\n");
        out.push_str("Result: source accepted by frontend pipeline\n");
    } else {
        out.push_str(&format!(
            "Status: typecheck failed with {} diagnostic(s)\n",
            typeck.diagnostics.len()
        ));
        for (idx, diag) in typeck.diagnostics.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", idx + 1, diag.message));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::repl_eval;

    #[test]
    fn repl_eval_rejects_empty_source() {
        let output = repl_eval("   \n\t");
        assert!(output.contains("error: source is empty"));
    }

    #[test]
    fn repl_eval_reports_success_for_valid_source() {
        let output = repl_eval("forge main() -> i32 { 42 }");
        assert!(output.contains("Status: typecheck passed"));
    }
}
