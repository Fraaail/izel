use crate::*;
use izel_parser::ast;
use izel_typeck::type_system::Type;

pub struct HirLowerer<'a> {
    pub resolver: &'a izel_resolve::Resolver,
    pub def_types: &'a rustc_hash::FxHashMap<izel_resolve::DefId, Type>,
}

impl<'a> HirLowerer<'a> {
    pub fn new(
        resolver: &'a izel_resolve::Resolver,
        def_types: &'a rustc_hash::FxHashMap<izel_resolve::DefId, Type>,
    ) -> Self {
        Self {
            resolver,
            def_types,
        }
    }

    fn get_def_id(&self, span: Span) -> DefId {
        self.resolver
            .def_ids
            .read()
            .unwrap()
            .get(&span)
            .cloned()
            .unwrap_or(DefId(0))
    }

    fn get_type(&self, def_id: DefId) -> Type {
        self.def_types.get(&def_id).cloned().unwrap_or(Type::Error)
    }

    pub fn lower_module(&self, module: &ast::Module) -> HirModule {
        let mut items = Vec::new();
        for item in &module.items {
            self.lower_item_to_vec(item, &mut items);
        }
        HirModule { items }
    }

    fn lower_item_to_vec(&self, item: &ast::Item, items: &mut Vec<HirItem>) {
        match item {
            ast::Item::Forge(f) => items.push(HirItem::Forge(Box::new(self.lower_forge(f)))),
            ast::Item::Shape(s) => items.push(HirItem::Shape(self.lower_shape(s))),
            ast::Item::Scroll(s) => items.push(HirItem::Scroll(self.lower_scroll(s))),
            ast::Item::Echo(e) => items.push(HirItem::Echo(self.lower_echo(e))),
            ast::Item::Dual(d) => {
                for inner in &d.items {
                    self.lower_item_to_vec(inner, items);
                }
            }
            ast::Item::Ward(w) => items.push(HirItem::Ward(self.lower_ward(w))),
            ast::Item::Draw(d) => items.push(HirItem::Draw(self.lower_draw(d))),
            _ => {}
        }
    }

    fn lower_shape(&self, shape: &ast::Shape) -> HirShape {
        HirShape {
            name: shape.name.clone(),
            def_id: self.get_def_id(shape.span),
            span: shape.span,
        }
    }

    fn lower_scroll(&self, scroll: &ast::Scroll) -> HirScroll {
        HirScroll {
            name: scroll.name.clone(),
            def_id: self.get_def_id(scroll.span),
            span: scroll.span,
        }
    }

    fn lower_echo(&self, echo: &ast::Echo) -> HirEcho {
        HirEcho {
            body: self.lower_block(&echo.body),
            span: echo.span,
        }
    }

    fn lower_ward(&self, ward: &ast::Ward) -> HirWard {
        let mut items = Vec::new();
        for item in &ward.items {
            self.lower_item_to_vec(item, &mut items);
        }
        HirWard {
            name: ward.name.clone(),
            items,
            span: ward.span,
        }
    }

    fn lower_draw(&self, draw: &ast::Draw) -> HirDraw {
        HirDraw {
            path: draw.path.clone(),
            def_id: None,
            is_wildcard: draw.is_wildcard,
            span: draw.span,
        }
    }

    fn lower_forge(&self, forge: &ast::Forge) -> HirForge {
        let forge_def_id = self.get_def_id(forge.name_span);
        let full_ty = self.get_type(forge_def_id);
        let ret_type = match full_ty {
            Type::Function { ret, .. } => *ret,
            _ => Type::Error,
        };
        HirForge {
            name: forge.name.clone(),
            name_span: forge.name_span,
            def_id: forge_def_id,
            params: forge.params.iter().map(|p| self.lower_param(p)).collect(),
            ret_type,
            attributes: forge.attributes.clone(),
            body: forge.body.as_ref().map(|b| self.lower_block(b)),
            requires: forge.requires.iter().map(|e| self.lower_expr(e)).collect(),
            ensures: forge.ensures.iter().map(|e| self.lower_expr(e)).collect(),
            span: forge.span,
        }
    }

    fn lower_param(&self, param: &ast::Param) -> HirParam {
        let def_id = self.get_def_id(param.span);
        let ty = self.get_type(def_id);
        HirParam {
            name: param.name.clone(),
            def_id,
            ty,
            default_value: param.default_value.as_ref().map(|e| self.lower_expr(e)),
            is_variadic: param.is_variadic,
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
            ast::Stmt::Let {
                pat, init, span, ..
            } => {
                let (name, name_span) = match pat {
                    ast::Pattern::Ident(name, _, span) => (name.clone(), *span),
                    _ => ("_hir_pattern_unsupported".to_string(), *span),
                };
                let def_id = self.get_def_id(name_span);

                let ty = self.get_type(def_id);
                eprintln!("HIR Let: name={}, def_id={:?}, ty={:?}", name, def_id, ty);

                HirStmt::Let {
                    name,
                    def_id,
                    ty,
                    init: init.as_ref().map(|e| self.lower_expr(e)),
                    span: *span,
                }
            }
            ast::Stmt::Expr(e) => HirStmt::Expr(self.lower_expr(e)),
        }
    }

    fn lower_expr(&self, expr: &ast::Expr) -> HirExpr {
        match expr {
            ast::Expr::Literal(lit) => HirExpr::Literal(lit.clone()),
            ast::Expr::Ident(name, span) => {
                let def_id = self.get_def_id(*span);
                HirExpr::Ident(name.clone(), def_id, self.get_type(def_id), *span)
            }
            ast::Expr::Binary(op, left, right) => HirExpr::Binary(
                op.clone(),
                Box::new(self.lower_expr(left)),
                Box::new(self.lower_expr(right)),
                Type::Error,
            ),
            ast::Expr::Unary(op, inner) => {
                HirExpr::Unary(op.clone(), Box::new(self.lower_expr(inner)), Type::Error)
            }
            ast::Expr::Call(callee, args) => {
                let callee_hir = self.lower_expr(callee);
                let mut ret_type = Type::Error;
                if let HirExpr::Ident(_, def_id, _, _) = &callee_hir {
                    if let Type::Function { ret, .. } = self.get_type(*def_id) {
                        ret_type = (*ret).clone();
                    }
                }
                HirExpr::Call(
                    Box::new(callee_hir),
                    args.iter().map(|a| self.lower_expr(&a.value)).collect(),
                    vec![],
                    ret_type,
                )
            }
            ast::Expr::Member(inner, name, span) => {
                let def_id = self.get_def_id(*span);
                HirExpr::Call(
                    Box::new(HirExpr::Ident(
                        name.clone(),
                        def_id,
                        self.get_type(def_id),
                        *span,
                    )),
                    vec![self.lower_expr(inner)],
                    vec![],
                    Type::Error, // Member access return type handled by typeck later or can be looked up
                )
            }
            ast::Expr::Given {
                cond,
                then_block,
                else_expr,
            } => HirExpr::Given {
                cond: Box::new(self.lower_expr(cond)),
                then_block: self.lower_block(then_block),
                else_expr: else_expr.as_ref().map(|e| Box::new(self.lower_expr(e))),
                ty: Type::Error,
            },
            ast::Expr::While { cond, body } => HirExpr::While {
                cond: Box::new(self.lower_expr(cond)),
                body: self.lower_block(body),
            },
            ast::Expr::Return(e) => HirExpr::Return(Some(Box::new(self.lower_expr(e)))),
            ast::Expr::Zone { name, body } => HirExpr::Zone {
                name: name.clone(),
                body: self.lower_block(body),
                ty: Type::Error,
            },
            ast::Expr::StructLiteral { .. } => HirExpr::Literal(ast::Literal::Nil), // Stub
            _ => HirExpr::Literal(ast::Literal::Nil),
        }
    }
}
