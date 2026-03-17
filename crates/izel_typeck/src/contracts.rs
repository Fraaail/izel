use crate::eval::{eval_expr, ConstValue};
use izel_parser::ast::{Forge, Expr};
use izel_diagnostics::{Diagnostic, primary_label};
use izel_span::Span;
use std::collections::HashMap;

pub struct ContractChecker;

impl ContractChecker {
    pub fn check_requires(forge: &Forge, args: &[ConstValue], call_span: Span) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut context = HashMap::new();
        
        for (param, val) in forge.params.iter().zip(args) {
            context.insert(param.name.clone(), val.clone());
        }

        for (i, req) in forge.requires.iter().enumerate() {
            let res = eval_expr(req, &context);
            if let ConstValue::Bool(false) = res {
                // Try to get custom message from attribute if we had it, 
                // but for now we only have Expr.
                diagnostics.push(Diagnostic::error()
                    .with_message(format!("precondition violation for '{}'", forge.name))
                    .with_code(format!("E-REQ-{}", i))
                    .with_labels(vec![primary_label(call_span, "requires condition not met")]));
            }
        }
        diagnostics
    }

    pub fn check_ensures(forge: &Forge, ret_val: &ConstValue, ret_span: Span, params: &HashMap<String, ConstValue>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut context = params.clone();
        
        // Match 'result' in ensures
        context.insert("result".to_string(), ret_val.clone());

        for (i, ens) in forge.ensures.iter().enumerate() {
            let res = eval_expr(ens, &context);
            if let ConstValue::Bool(false) = res {
                diagnostics.push(Diagnostic::error()
                    .with_message(format!("postcondition violation for '{}'", forge.name))
                    .with_code(format!("E-ENS-{}", i))
                    .with_labels(vec![primary_label(ret_span, "ensures condition not met")]));
            }
        }
        diagnostics
    }
}
