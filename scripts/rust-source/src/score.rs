//! Independent source metric. Macro token trees are deliberately opaque.
use syn::visit::{self, Visit};
use syn::{BinOp, Block, Expr};

#[derive(Default)]
pub struct Score(pub usize);

pub fn nonempty(expr: &Expr) -> bool {
    match expr {
        Expr::Block(value) => !value.block.stmts.is_empty(),
        _ => true,
    }
}

impl<'ast> Visit<'ast> for Score {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        self.0 += usize::from(!node.block.stmts.is_empty());
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        self.0 += usize::from(!node.block.stmts.is_empty());
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(body) = &node.default {
            self.0 += usize::from(!body.stmts.is_empty());
        }
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        self.0 += match node {
            Expr::If(_) | Expr::Loop(_) | Expr::While(_) | Expr::ForLoop(_) | Expr::Closure(_) => 1,
            Expr::Binary(value) => usize::from(matches!(value.op, BinOp::And(_) | BinOp::Or(_))),
            _ => 0,
        };
        visit::visit_expr(self, node);
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        self.0 += usize::from(nonempty(&node.body));
        visit::visit_arm(self, node);
    }
}

pub fn function(sig: &syn::Signature, block: &Block) -> usize {
    let mut score = Score(usize::from(!block.stmts.is_empty()));
    score.visit_signature(sig);
    score.visit_block(block);
    score.0
}
