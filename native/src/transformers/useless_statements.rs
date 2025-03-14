use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_ecma_ast::Program;

pub struct Visitor;

impl VisitMut for Visitor {
    fn visit_mut_stmts(&mut self, n: &mut std::vec::Vec<swc_ecma_ast::Stmt>) {
        n.visit_mut_children_with(self);
        let mut new_stmtns: std::vec::Vec<swc_ecma_ast::Stmt> = vec![];

        for stmt in &n.to_owned() {
            //remove useless statements like this: gF;
            if stmt.is_expr() {
                let expr_stmt = &stmt.as_expr().unwrap();
                if expr_stmt.expr.is_ident() {
                    continue;
                }
            }
            new_stmtns.push(stmt.to_owned());
        }
        *n = new_stmtns;
    }
    fn visit_mut_program(&mut self, n: &mut Program) {
        println!("[*] Replacing useless statements: gF;");
        n.visit_mut_children_with(self);
    }
}
