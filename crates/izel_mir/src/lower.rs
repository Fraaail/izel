use crate::*;
use izel_hir::{HirForge, HirBlock, HirStmt, HirExpr};
use izel_typeck::type_system::Type;
use izel_resolve::DefId;
use std::collections::HashMap;

pub struct MirLowerer {
    body: MirBody,
    current_block: BlockId,
    header: BlockId,
    forge_name: String,
    param_defs: Vec<DefId>,
    /// Maps source DefId to its current SSA Local version in a given block.
    current_defs: HashMap<(DefId, BlockId), Local>,
    /// Tracks which blocks are "sealed" (all predecessors known).
    sealed_blocks: Vec<BlockId>,
    /// Incomplete Phis: (BlockId, DefId, Local_assigned_to_phi)
    incomplete_phis: HashMap<BlockId, Vec<(DefId, Local)>>,
    pub check_contracts: bool,
}

impl MirLowerer {
    pub fn new() -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self {
            body,
            current_block: entry,
            header: entry,
            forge_name: String::new(),
            param_defs: Vec::new(),
            current_defs: HashMap::new(),
            sealed_blocks: Vec::new(),
            incomplete_phis: HashMap::new(),
            check_contracts: false,
        }
    }

    pub fn lower_forge(&mut self, forge: &HirForge) -> MirBody {
        self.current_defs.clear();
        self.sealed_blocks.clear();
        self.incomplete_phis.clear();
        self.forge_name = forge.name.clone();
        self.param_defs = forge.params.iter().map(|p| p.def_id).collect();

        let entry = self.body.entry;
        let header = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
        self.header = header;

        // Lower parameters as initial definitions in entry
        for param in &forge.params {
            let local = self.new_local(param.name.clone(), param.ty.clone());
            self.write_variable(param.def_id, entry, local);
        }

        self.body.blocks.add_edge(entry, header, ControlFlow::Unconditional);
        self.body.blocks[entry].terminator = Some(Terminator::Goto(header));
        self.seal_block(entry);
        
        self.current_block = header;

        if let Some(body) = &forge.body {
            self.lower_block(body);
        }

        self.seal_block(self.header);

        let block = &mut self.body.blocks[self.current_block];
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Return);
        }

        std::mem::replace(&mut self.body, MirBody::new())
    }

    fn new_local(&mut self, name: String, ty: Type) -> Local {
        let id = self.body.locals.len();
        self.body.locals.push(LocalData { name, ty });
        Local(id)
    }

    fn write_variable(&mut self, var: DefId, block: BlockId, local: Local) {
        self.current_defs.insert((var, block), local);
    }

    fn read_variable(&mut self, var: DefId, block: BlockId) -> Local {
        if let Some(&local) = self.current_defs.get(&(var, block)) {
            local
        } else {
            self.read_variable_recursive(var, block)
        }
    }

    fn read_variable_recursive(&mut self, var: DefId, block: BlockId) -> Local {
        let local;
        if !self.sealed_blocks.contains(&block) {
            local = self.new_local(format!("phi_v{:?}", var), Type::Error);
            self.incomplete_phis.entry(block).or_default().push((var, local));
        } else {
            let preds: Vec<_> = self.body.blocks.neighbors_directed(block, petgraph::Direction::Incoming).collect();
            if preds.len() == 1 {
                local = self.read_variable(var, preds[0]);
            } else if preds.is_empty() {
                // Should not happen for reachable blocks except entry
                local = self.new_local("undef".into(), Type::Error);
            } else {
                local = self.new_local(format!("phi_v{:?}", var), Type::Error);
                self.write_variable(var, block, local);
                self.add_phi_operands(var, local, block);
            }
        }
        self.write_variable(var, block, local);
        local
    }

    fn add_phi_operands(&mut self, var: DefId, phi_local: Local, block: BlockId) {
        let preds: Vec<_> = self.body.blocks.neighbors_directed(block, petgraph::Direction::Incoming).collect();
        let mut operands = Vec::new();
        for pred in preds {
            let val = self.read_variable(var, pred);
            operands.push((pred, val));
        }
        // Insert Phi at the beginning
        self.body.blocks[block].instructions.insert(0, Instruction::Phi(phi_local, operands));
    }

    pub fn seal_block(&mut self, block: BlockId) {
        if let Some(phis) = self.incomplete_phis.remove(&block) {
            for (var, phi_local) in phis {
                self.add_phi_operands(var, phi_local, block);
            }
        }
        self.sealed_blocks.push(block);
    }

    fn lower_block(&mut self, block: &HirBlock) {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
        }
        if let Some(expr) = &block.expr {
            self.lower_expr(expr);
        }
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Let { name, def_id, ty, init, .. } => {
                if let Some(val_expr) = init {
                    let rvalue = self.lower_expr(val_expr);
                    let local = self.new_local(name.clone(), ty.clone());
                    self.body.blocks[self.current_block].instructions.push(Instruction::Assign(local, rvalue));
                    self.write_variable(*def_id, self.current_block, local);
                }
            }
            HirStmt::Assign { def_id, expr, .. } => {
                let rvalue = self.lower_expr(expr);
                let local = self.new_local(format!("reassign_{:?}", def_id), Type::Error);
                self.body.blocks[self.current_block].instructions.push(Instruction::Assign(local, rvalue));
                self.write_variable(*def_id, self.current_block, local);
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Rvalue {
        match expr {
            HirExpr::Literal(lit) => {
                let constant = match lit {
                    izel_parser::ast::Literal::Int(v) => Constant::Int(*v),
                    izel_parser::ast::Literal::Float(v) => Constant::Float(*v),
                    izel_parser::ast::Literal::Bool(v) => Constant::Bool(*v),
                    izel_parser::ast::Literal::Str(s) => Constant::Str(s.clone()),
                    izel_parser::ast::Literal::Nil => Constant::Bool(false),
                };
                Rvalue::Use(Operand::Constant(constant))
            }
            HirExpr::Ident(def_id, _, _) => {
                let local = self.read_variable(*def_id, self.current_block);
                Rvalue::Use(Operand::Move(local))
            }
            HirExpr::Binary(op, left, right, _) => {
                let lr = self.lower_expr(left);
                let l_op = self.rvalue_to_operand(lr);
                let rr = self.lower_expr(right);
                let r_op = self.rvalue_to_operand(rr);
                
                let mir_op = match op {
                    izel_parser::ast::BinaryOp::Add => BinOp::Add,
                    _ => BinOp::Add, // map other ops...
                };
                Rvalue::BinaryOp(mir_op, l_op, r_op)
            }
            HirExpr::Call(callee, args, _) => {
                let mut operands = Vec::new();
                for arg in args {
                    let rv = self.lower_expr(arg);
                    operands.push(self.rvalue_to_operand(rv));
                }
                let local = self.new_local("call_tmp".to_string(), Type::Error);
                self.body.blocks[self.current_block].instructions.push(Instruction::Call(local, "unknown".to_string(), operands));
                Rvalue::Use(Operand::Move(local))
            }
            HirExpr::Return(expr) => {
                if let Some(e) = expr {
                    if let HirExpr::Call(callee, args, _) = &**e {
                        // Check for TCO
                        let is_recursive = if let HirExpr::Ident(_, _, _) = &**callee { true } else { false };
                        if is_recursive {
                            // TCO transformation:
                            let mut arg_ops = Vec::new();
                            for arg in args {
                                let rv = self.lower_expr(arg);
                                arg_ops.push(self.rvalue_to_operand(rv));
                            }
                            // Re-assign params
                            let param_defs = self.param_defs.clone();
                            for (i, def_id) in param_defs.iter().enumerate() {
                                if i < arg_ops.len() {
                                    let local = self.new_local(format!("tco_p{}", i), Type::Error);
                                    self.body.blocks[self.current_block].instructions.push(Instruction::Assign(local, Rvalue::Use(arg_ops[i].clone())));
                                    self.write_variable(*def_id, self.current_block, local);
                                }
                            }
                            self.body.blocks.add_edge(self.current_block, self.header, ControlFlow::Unconditional);
                            self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(self.header));
                            return Rvalue::Use(Operand::Constant(Constant::Int(0)));
                        }
                    }
                    let rv = self.lower_expr(e);
                    let op = self.rvalue_to_operand(rv);
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Return);
                    Rvalue::Use(op)
                } else {
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Return);
                    Rvalue::Use(Operand::Constant(Constant::Int(0)))
                }
            }
            HirExpr::Given { cond, then_block, else_expr, .. } => {
                let cond_rv = self.lower_expr(cond);
                let cond_op = self.rvalue_to_operand(cond_rv);

                let then_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let else_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });
                let join_id = self.body.blocks.add_node(BasicBlock { instructions: Vec::new(), terminator: None });

                self.body.blocks.add_edge(self.current_block, then_id, ControlFlow::Conditional(true));
                self.body.blocks.add_edge(self.current_block, else_id, ControlFlow::Conditional(false));
                self.body.blocks[self.current_block].terminator = Some(Terminator::SwitchInt(cond_op, vec![(1, then_id)], else_id));

                self.seal_block(then_id);
                self.seal_block(else_id);

                self.current_block = then_id;
                self.lower_block(then_block);
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(self.current_block, join_id, ControlFlow::Unconditional);
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(join_id));
                }

                self.current_block = else_id;
                if let Some(el) = else_expr {
                    self.lower_expr(el);
                }
                if self.body.blocks[self.current_block].terminator.is_none() {
                    self.body.blocks.add_edge(self.current_block, join_id, ControlFlow::Unconditional);
                    self.body.blocks[self.current_block].terminator = Some(Terminator::Goto(join_id));
                }

                self.current_block = join_id;
                self.seal_block(join_id);
                Rvalue::Use(Operand::Constant(Constant::Int(0)))
            }
            _ => Rvalue::Use(Operand::Constant(Constant::Int(0))),
        }
    }

    fn rvalue_to_operand(&mut self, rvalue: Rvalue) -> Operand {
        match rvalue {
            Rvalue::Use(op) => op,
            _ => {
                let local = self.new_local("tmp".to_string(), Type::Error);
                self.body.blocks[self.current_block].instructions.push(Instruction::Assign(local, rvalue));
                Operand::Move(local)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use izel_hir::*;
    use izel_span::Span;

    #[test]
    fn test_lower_ssa_let() {
        let forge = HirForge {
            name: "main".into(),
            def_id: DefId(0),
            params: vec![],
            ret_type: Type::Error,
            body: Some(HirBlock {
                stmts: vec![
                    HirStmt::Let {
                        name: "x".into(),
                        def_id: DefId(10),
                        ty: Type::Error,
                        init: Some(HirExpr::Literal(izel_parser::ast::Literal::Int(1))),
                        span: Span::dummy(),
                    },
                ],
                expr: None,
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };
        let mut lowerer = MirLowerer::new();
        let _mir = lowerer.lower_forge(&forge);
    }

    #[test]
    fn test_tco() {
        let x_def = DefId(10);
        let forge = HirForge {
            name: "fact".into(),
            def_id: DefId(0),
            params: vec![HirParam { name: "n".into(), def_id: x_def, ty: Type::Error, span: Span::dummy() }],
            ret_type: Type::Error,
            body: Some(HirBlock {
                stmts: vec![
                ],
                expr: Some(Box::new(HirExpr::Return(Some(Box::new(HirExpr::Call(
                    Box::new(HirExpr::Ident(DefId(0), Type::Error, Span::dummy())),
                    vec![HirExpr::Literal(izel_parser::ast::Literal::Int(0))],
                    Type::Error
                )))))),
                span: Span::dummy(),
            }),
            requires: vec![],
            ensures: vec![],
            span: Span::dummy(),
        };

        let mut lowerer = MirLowerer::new();
        let mir = lowerer.lower_forge(&forge);

        // Check for back-edge to header
        let mut has_back_edge = false;
        for edge in mir.blocks.edge_indices() {
            let (u, v) = mir.blocks.edge_endpoints(edge).unwrap();
            // Header is index 1.
            if v.index() == 1 && u.index() >= 1 {
                has_back_edge = true;
            }
        }
        if !has_back_edge {
            for node in mir.blocks.node_indices() {
                println!("Block {:?}: {:?}", node, mir.blocks[node].terminator);
            }
        }
        assert!(has_back_edge, "TCO should have created a back-edge to the header block");
    }
}
