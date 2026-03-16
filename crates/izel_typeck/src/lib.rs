pub mod type_system;
pub use type_system::{Type, PrimType};
use izel_resolve::DefId;
use izel_parser::ast;
use rustc_hash::FxHashMap;

pub struct TypeChecker {
    /// Resolved types for each DefId
    pub def_types: FxHashMap<DefId, Type>,
    /// Type of each expression span/id (once we have Expr IDs)
    pub expr_types: FxHashMap<usize, Type>,
    pub substitutions: FxHashMap<usize, Type>,
    pub env: Vec<FxHashMap<String, Type>>,
    next_var: usize,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            def_types: FxHashMap::default(),
            expr_types: FxHashMap::default(),
            substitutions: FxHashMap::default(),
            env: vec![FxHashMap::default()], // Global scope
            next_var: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.env.push(FxHashMap::default());
    }

    pub fn pop_scope(&mut self) {
        self.env.pop();
    }

    pub fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.env.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub fn resolve_name(&self, name: &str) -> Option<Type> {
        for scope in self.env.iter().rev() {
            if let Some(ty) = scope.get(name) {
                // println!("Resolved {} to {:?}", name, ty);
                return Some(ty.clone());
            }
        }
        None
    }

    pub fn new_var(&mut self) -> Type {
        let var = Type::Var(self.next_var);
        self.next_var += 1;
        var
    }

    pub fn check_ast(&mut self, module: &ast::Module) {
        // Pass 1: Collect top-level item signatures (simplified for now)
        for item in &module.items {
            self.collect_item_signature(item);
        }

        // Pass 2: Check bodies
        for item in &module.items {
            match item {
                ast::Item::Forge(f) => {
                     self.push_scope();
                     for param in &f.params {
                          let pty = self.lower_ast_type(&param.ty);
                          self.define(param.name.clone(), pty);
                     }
                     if let Some(body) = &f.body {
                          // No need to push another scope for the main block if we reuse this one
                          // But check_block normally pushes one. 
                          // Let's just define params in the "forge" scope and let check_block push its own.
                          self.check_block(body);
                     }
                     self.pop_scope();
                }
                _ => {}
            }
        }
    }

    fn collect_item_signature(&mut self, item: &ast::Item) {
        match item {
            ast::Item::Forge(f) => {
                let params = f.params.iter().map(|p| self.lower_ast_type(&p.ty)).collect();
                let ret = Box::new(self.lower_ast_type(&f.ret_type));
                self.define(f.name.clone(), Type::Function { params, ret });
            }
            ast::Item::Shape(s) => {
                let mut fields = vec![];
                for f in &s.fields {
                    fields.push((f.name.clone(), self.lower_ast_type(&f.ty)));
                }
                self.define(s.name.clone(), Type::Static(fields)); // Simplified to Static for now
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &ast::Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.infer_expr(expr);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &ast::Stmt) {
        match stmt {
            ast::Stmt::Expr(e) => { self.infer_expr(e); }
            ast::Stmt::Let { name, ty, init, span: _ } => {
                let mut var_ty = self.new_var();
                if let Some(explicit_ty) = ty {
                    let et = self.lower_ast_type(explicit_ty);
                    self.unify(&var_ty, &et);
                    var_ty = et;
                }
                if let Some(init_expr) = init {
                    let it = self.infer_expr(init_expr);
                    self.unify(&var_ty, &it);
                    // If no explicit type, the var_ty becomes it (via unification)
                }
                self.define(name.clone(), var_ty);
            }
        }
    }

    fn lower_ast_type(&self, ty: &ast::Type) -> Type {
        match ty {
            ast::Type::Prim(s) => match s.as_str() {
                "i32" => Type::Prim(PrimType::I32),
                "str" => Type::Prim(PrimType::Str),
                "bool" => Type::Prim(PrimType::Bool),
                "void" => Type::Prim(PrimType::Void),
                _ => {
                    if let Some(t) = self.resolve_name(s) {
                        t
                    } else {
                        Type::Error
                    }
                }
            },
            ast::Type::Optional(inner) => Type::Optional(Box::new(self.lower_ast_type(inner))),
            ast::Type::Cascade(inner) => Type::Cascade(Box::new(self.lower_ast_type(inner))),
            ast::Type::Pointer(inner, m) => Type::Pointer(Box::new(self.lower_ast_type(inner)), *m),
            _ => Type::Error,
        }
    }

    pub fn unify(&mut self, t1: &Type, t2: &Type) -> bool {
        let t1 = self.prune(t1);
        let t2 = self.prune(t2);

        match (&t1, &t2) {
            (Type::Var(id1), Type::Var(id2)) if id1 == id2 => true,
            (Type::Var(id), other) => {
                self.bind_var(*id, (*other).clone());
                true
            }
            (other, Type::Var(id)) => {
                self.bind_var(*id, (*other).clone());
                true
            }
            (Type::Prim(p1), Type::Prim(p2)) => p1 == p2,
            (Type::Static(f1), Type::Static(f2)) => {
                if f1.len() != f2.len() { return false; }
                for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                    if n1 != n2 || !self.unify(t1, t2) { return false; }
                }
                true
            }
            (Type::Optional(o1), Type::Optional(o2)) => self.unify(&o1, &o2),
            (Type::Cascade(c1), Type::Cascade(c2)) => self.unify(&c1, &c2),
            (Type::Pointer(p1, m1), Type::Pointer(p2, m2)) => m1 == m2 && self.unify(&p1, &p2),
            _ => {
                false
            }
        }
    }

    fn prune(&self, ty: &Type) -> Type {
        if let Type::Var(id) = ty {
            if let Some(bound) = self.substitutions.get(id) {
                return self.prune(bound);
            }
        }
        ty.clone()
    }

    fn bind_var(&mut self, id: usize, ty: Type) {
        if let Type::Var(other_id) = ty {
             if id == other_id { return; }
        }
        self.substitutions.insert(id, ty);
    }

    pub fn infer_expr(&mut self, expr: &ast::Expr) -> Type {
        let res = match expr {
            ast::Expr::Literal(l) => match l {
                ast::Literal::Int(_) => Type::Prim(PrimType::I32),
                ast::Literal::Float(_) => Type::Prim(PrimType::F64),
                ast::Literal::Str(_) => Type::Prim(PrimType::Str),
                ast::Literal::Bool(_) => Type::Prim(PrimType::Bool),
                ast::Literal::Nil => Type::Prim(PrimType::None),
            },
            ast::Expr::Ident(name, _) => {
                if let Some(ty) = self.resolve_name(name) {
                    ty
                } else {
                    Type::Error
                }
            }
            ast::Expr::Binary(_, lhs, rhs) => {
                let lt = self.infer_expr(lhs);
                let rt = self.infer_expr(rhs);
                self.unify(&lt, &rt);
                lt
            }
            ast::Expr::Member(obj, field, _) => {
                let ot = self.infer_expr(obj);
                if let Type::Static(fields) = self.prune(&ot) {
                    if let Some((_, fty)) = fields.iter().find(|(name, _)| name == field) {
                         return fty.clone();
                    }
                }
                self.new_var()
            }
            ast::Expr::Call(callee, args) => {
                let ct = self.infer_expr(callee);
                if let Type::Function { params, ret } = self.prune(&ct) {
                     for (arg, pty) in args.iter().zip(params.iter()) {
                          let at = self.infer_expr(arg);
                          self.unify(&at, pty);
                     }
                     *ret
                } else {
                     for arg in args { self.infer_expr(arg); }
                     self.new_var()
                }
            }
            ast::Expr::StructLiteral { path, fields } => {
                let mut struct_ty = self.new_var();
                if let ast::Type::Prim(name) = path {
                     if let Some(ty) = self.resolve_name(name) {
                          struct_ty = ty;
                     }
                }
                if let Type::Static(st_fields) = self.prune(&struct_ty) {
                     for (fname, fexpr) in fields {
                          if let Some((_, fty)) = st_fields.iter().find(|(n, _)| n == fname) {
                               let et = self.infer_expr(fexpr);
                               self.unify(&et, fty);
                          }
                     }
                }
                struct_ty
            }
            ast::Expr::Branch { target, arms } => {
                let tt = self.infer_expr(target);
                let res_ty = self.new_var();
                for arm in arms {
                    self.push_scope();
                    self.bind_pattern(&arm.pattern, &tt);
                    let at = self.infer_expr(&arm.body);
                    self.unify(&res_ty, &at);
                    self.pop_scope();
                }
                res_ty
            }
            _ => self.new_var(),
        };
        // TODO: Store in expr_types
        res
    }

    fn bind_pattern(&mut self, pattern: &ast::Pattern, ty: &Type) {
        let ty = self.prune(ty);
        match pattern {
            ast::Pattern::Ident(name) => {
                self.define(name.clone(), ty);
            }
            ast::Pattern::Variant(variant, subpatterns) => {
                // Hardcoded logic for Optional and Cascade unwrapping for now
                match ty {
                    Type::Optional(inner) | Type::Cascade(inner) => {
                        for sub in subpatterns {
                            self.bind_pattern(sub, &inner);
                        }
                    }
                    _ => {
                        for sub in subpatterns {
                            self.bind_pattern(sub, &Type::Error);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
