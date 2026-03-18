use crate::*;
use izel_parser::ast;
use izel_typeck::type_system::Type;

pub struct HirLowerer {
    // In a real compiler, this would be populated by the Resolver
}

impl HirLowerer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn lower_module(&self, module: &ast::Module) -> HirModule {
        HirModule {
            items: module.items.iter().filter_map(|item| self.lower_item(item)).collect(),
        }
    }

    fn lower_item(&self, item: &ast::Item) -> Option<HirItem> {
        match item {
            ast::Item::Forge(f) => Some(HirItem::Forge(self.lower_forge(f))),
            _ => None,
        }
    }

    fn lower_forge(&self, forge: &ast::Forge) -> HirForge {
        HirForge {
            name: forge.name.clone(),
            def_id: izel_resolve::DefId(0), // Mock
            params: forge.params.iter().map(|p| self.lower_param(p)).collect(),
            ret_type: Type::Error,
            body: forge.body.as_ref().map(|b| self.lower_block(b)),
            requires: forge.requires.iter().map(|e| self.lower_expr(e)).collect(),
            ensures: forge.ensures.iter().map(|e| self.lower_expr(e)).collect(),
            span: forge.span,
        }
    }

    fn lower_param(&self, param: &ast::Param) -> HirParam {
         HirParam {
            name: param.name.clone(),
            def_id: izel_resolve::DefId(1), // Mock
            ty: Type::Error,
            span: param.span,
        }
    }

    fn lower_block(&self, block: &ast::Block) -> HirBlock {
        HirBlock {
            stmts: block.stmts.iter().map(|s| self.lower_stmt(s)).collect(),
            expr: block.expr.as_ref().map(|e| Box::new(self.lower_expr(e))),
            span: block.span,
        }
    }

    fn lower_stmt(&self, stmt: &ast::Stmt) -> HirStmt {
        match stmt {
            ast::Stmt::Let { name, init, span, .. } => HirStmt::Let {
                name: name.clone(),
                def_id: izel_resolve::DefId(2), // Mock
                ty: Type::Error,
                init: init.as_ref().map(|e| self.lower_expr(e)),
                span: *span,
            },
            ast::Stmt::Expr(e) => HirStmt::Expr(self.lower_expr(e)),
        }
    }

    fn lower_expr(&self, expr: &ast::Expr) -> HirExpr {
        match expr {
            ast::Expr::Literal(lit) => HirExpr::Literal(lit.clone()),
            ast::Expr::Ident(_name, span) => HirExpr::Ident(izel_resolve::DefId(3), Type::Error, *span),
            ast::Expr::Binary(op, left, right) => HirExpr::Binary(
                op.clone(),
                Box::new(self.lower_expr(left)),
                Box::new(self.lower_expr(right)),
                Type::Error,
            ),
            ast::Expr::Unary(op, inner) => HirExpr::Unary(
                op.clone(),
                Box::new(self.lower_expr(inner)),
                Type::Error,
            ),
            ast::Expr::Call(callee, args) => HirExpr::Call(
                Box::new(self.lower_expr(callee)),
                args.iter().map(|a| self.lower_expr(a)).collect(),
                Type::Error,
            ),
            ast::Expr::Given { cond, then_block, else_expr } => HirExpr::Given {
                cond: Box::new(self.lower_expr(cond)),
                then_block: self.lower_block(then_block),
                else_expr: else_expr.as_ref().map(|e| Box::new(self.lower_expr(e))),
                ty: Type::Error,
            },
            ast::Expr::While { cond, body } => HirExpr::While {
                cond: Box::new(self.lower_expr(cond)),
                body: self.lower_block(body),
            },
            _ => HirExpr::Literal(ast::Literal::Nil),
        }
    }
}
