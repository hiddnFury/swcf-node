use swc_common::Spanned;
use swc_core::ecma::{
    utils::ExprExt,
    visit::{VisitMut, VisitMutWith},
};
use swc_ecma_ast::{BinExpr, BinaryOp, Program};

use crate::utils::utils;

pub struct Visitor;

impl VisitMut for Visitor {
    fn visit_mut_expr(&mut self, n: &mut swc_ecma_ast::Expr) {
        n.visit_mut_children_with(self);

        if n.is_bin() {
            let bin = n.as_bin().unwrap();
            if bin.op == BinaryOp::BitXor || bin.op == BinaryOp::BitAnd || bin.op == BinaryOp::BitOr {

                let mut right = bin.right.to_owned();
                let mut left = bin.left.to_owned();
                let mut reversed = false;
    
                if bin.right.is_number() {
                    let num_value = utils::number_from_lit(bin.right.as_lit().unwrap());
                    right = num_value.floor().into();
                    reversed = true;
                }
    
                if bin.left.is_number() {
                    let num_value = utils::number_from_lit(bin.left.as_lit().unwrap());
                    left = num_value.floor().into();
                }
    
                let mut bin_expr = BinExpr {
                    span: n.span(),
                    op: bin.op,
                    left: left,
                    right: right,
                };
                if reversed {
                    let tmp = bin_expr.left;
                    bin_expr.left = bin_expr.right;
                    bin_expr.right = tmp;
                }
                *n = swc_ecma_ast::Expr::Bin(
                    swc_ecma_ast::Expr::bin(swc_ecma_ast::Expr::Bin(bin_expr)).unwrap(),
                )
            }
            else if bin.op == BinaryOp::LShift || bin.op == BinaryOp::RShift {
                let mut right = bin.right.to_owned();
                let mut left = bin.left.to_owned();

                if bin.right.is_number() {
                    let num_value = utils::number_from_lit(bin.right.as_lit().unwrap());
                    right = num_value.floor().into();
                }

                if bin.left.is_number() {
                    let num_value = utils::number_from_lit(bin.left.as_lit().unwrap());
                    left = num_value.floor().into();
                }

                let mut bin_expr = BinExpr {
                    span: n.span(),
                    op: bin.op,
                    left: left,
                    right: right,
                };

                *n = swc_ecma_ast::Expr::Bin(
                    swc_ecma_ast::Expr::bin(swc_ecma_ast::Expr::Bin(bin_expr)).unwrap(),
                )
            }
            else {
                return;
            }
        }
    }
    fn visit_mut_program(&mut self, n: &mut Program) {
        println!("[*] Simplifying binary expressions");
        n.visit_mut_children_with(self);
    }
}
