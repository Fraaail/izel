use crate::*;
use izel_parser::cst::{SyntaxNode, NodeKind, SyntaxElement};
use izel_lexer::TokenKind;

pub struct MirLowerer<'a> {
    source: &'a str,
    body: MirBody,
    current_block: BlockId,
}

impl<'a> MirLowerer<'a> {
    pub fn new(source: &'a str) -> Self {
        let body = MirBody::new();
        let entry = body.entry;
        Self { source, body, current_block: entry }
    }

    pub fn lower_forge(&mut self, node: &SyntaxNode) -> MirBody {
        // Find the block
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                if child_node.kind == NodeKind::Block {
                    self.lower_block(child_node);
                }
            }
        }
        
        // Ensure terminator
        let block = self.body.blocks.node_weight_mut(self.current_block).unwrap();
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Return);
        }

        std::mem::replace(&mut self.body, MirBody::new())
    }

    fn lower_block(&mut self, node: &SyntaxNode) {
        for child in &node.children {
            if let SyntaxElement::Node(child_node) = child {
                match child_node.kind {
                    NodeKind::LetStmt => {
                        // TODO: Handle let properly
                    }
                    NodeKind::ExprStmt => {
                        // TODO: Handle expr
                    }
                    _ => {}
                }
            }
        }
    }
}
