use swc::atoms::Atom;
use swc_common::util::take::Take;
use swc_common::Span;
use swc_core::ecma::ast::Program;
use swc_core::ecma::visit::VisitMut;
use swc_ecma_ast::{AssignOp, BinExpr, BinaryOp, CallExpr, Expr, ParenExpr, Lit, Str};
use swc_ecma_visit::{Visit, VisitMutWith, VisitWith};

#[derive(Default)]
struct FindString {
    str: String,
}
impl Visit for FindString {
    fn visit_str(&mut self, n: &swc_ecma_ast::Str) {
        self.str = n.value.to_string()
    }
}

struct Proxy {
    proxy_type: String,
    key: String,
    string_value: String,
    bin_operator: BinaryOp,
    reversed: bool,
}

impl Proxy {
    pub fn string(key: String, value: String) -> Proxy {
        return Proxy {
            proxy_type: "string".to_owned(),
            key: key,
            string_value: value,
            bin_operator: BinaryOp::Add,
            reversed: false,
        };
    }
    pub fn call(key: String) -> Proxy {
        return Proxy {
            proxy_type: "call".to_owned(),
            key: key,
            string_value: "".to_owned(),
            bin_operator: BinaryOp::Add,
            reversed: false,
        };
    }
    pub fn binary(key: String, operator: BinaryOp, reversed: bool) -> Proxy {
        return Proxy {
            proxy_type: "binary".to_owned(),
            key: key,
            string_value: "".to_owned(),
            bin_operator: operator,
            reversed: reversed,
        };
    }
}

#[derive(Default)]
struct FindProxyAssignments {
    assignments: Vec<Proxy>,
}

impl VisitMut for FindProxyAssignments {
    // "abcdef": function() {}
    fn visit_mut_key_value_prop(&mut self, n: &mut swc_ecma_ast::KeyValueProp) {
        n.visit_mut_children_with(self);
        let key_opt = &n.key.as_str();
        if key_opt.to_owned().is_none() {
            return;
        }
        let key = &key_opt.unwrap().value;
        if key.len() != 5 || key == "event" || key == "kuMu1" {
            return;
        }

        let as_lit = n.value.as_lit();
        let as_fn = n.value.as_fn_expr();

        if as_lit.is_some() {
            let mut str = FindString::default();
            n.value.visit_children_with(&mut str);
            self.assignments
                .push(Proxy::string(key.to_string(), str.str));

            n.value.take();
            n.key.take();
        } else if as_fn.is_some() {
            let func = &as_fn.unwrap().function;
            let stmts = <Option<swc_ecma_ast::BlockStmt> as Clone>::clone(&func.body)
                .unwrap()
                .stmts;
            let return_stmt = stmts.first().unwrap().as_return_stmt().unwrap();
            let mut expr = <Option<Box<swc_ecma_ast::Expr>> as Clone>::clone(&return_stmt.arg).unwrap();
            // println!("visit_key_value_prop: Unsupported {} (function)", key,);
            let seq = expr.as_seq();
            if seq.is_some() {
                let seq = seq.unwrap();
                expr = seq.exprs.last().unwrap().to_owned();
            }
            let as_call = expr.as_call();
            let as_bin = expr.as_bin();
            if as_call.is_some() {
                // Handle special case:
                // e.pHFEm = function(i, j) {
                //     return i + j;
                // }
                // f.abCxY = function(b, c) {
                //   return e.PHFEm(b, c);
                // }
                // In this case, we need to handle the call as a binary operation
                // Check if the function is a call to another function
                let call = as_call.unwrap();
                let callee = call.callee.as_expr().unwrap();
                if let Some(as_member) = callee.as_member() {
                    let comp = as_member.prop.as_computed().unwrap();
                    let proxy_key = match comp.expr.as_lit().unwrap() {
                        Lit::Str(Str { value, .. }) => Some(value.to_string()),
                        _ => None, // Not a string literal
                    };

                    //Find Proxy with proxy_key in assignments
                    let proxy_key_str = proxy_key.clone().unwrap();
                    let maybe_p = self.assignments.iter().find(|p| p.key == proxy_key_str);
                    if maybe_p.is_none() {
                        return;
                    }
                    let p = maybe_p.unwrap();
                    
                    if p.proxy_type == "call" {
                        //check if assignment already exists for the key
                        if self.assignments.iter().find(|p| p.key == key.to_string()).is_some() {
                            println!("Assignment already exists for key: {}", key);
                            return;
                        }
                        self.assignments.push(Proxy::call(key.to_string()));
                    } else if p.proxy_type == "binary" {
                        //check if assignment already exists for the key
                        if self.assignments.iter().find(|p| p.key == key.to_string()).is_some() {
                            println!("Assignment already exists for key: {}", key);
                            return;
                        }
                        self.assignments
                            .push(Proxy::binary(key.to_string(), p.bin_operator, p.reversed));
                    }
                }
                else {
                    //check if assignment already exists for the key
                    if self.assignments.iter().find(|p| p.key == key.to_string()).is_some() {
                        println!("Assignment already exists for key: {}", key);
                        return;
                    }
                    self.assignments.push(Proxy::call(key.to_string()));
                }
                n.value.take();
                n.key.take();
            } else if as_bin.is_some() {
                let bin = as_bin.unwrap();
                //check if assignment already exists for the key
                if self.assignments.iter().find(|p| p.key == key.to_string()).is_some() {
                    println!("Assignment already exists for key: {}", key);
                    return;
                }
                let reversed = bin.right.as_ident().unwrap().sym.as_str()
                    == func
                        .params
                        .first()
                        .unwrap()
                        .pat
                        .as_ident()
                        .unwrap()
                        .sym
                        .as_str();
                self.assignments
                    .push(Proxy::binary(key.to_string(), bin.op, reversed));
                n.value.take();
                n.key.take();
            }
        } else {
            // println!("visit_key_value_prop {} {:?}", key, n.value);
        }
    }
    // e.pHFEm = "overlay",
    fn visit_mut_assign_expr(&mut self, n: &mut swc_ecma_ast::AssignExpr) {
        n.visit_mut_children_with(self);
        if n.op != AssignOp::Assign {
            return;
        }
        let right_lit = n.right.as_lit();
        let right_fun = n.right.as_fn_expr();

        let simple = n.left.as_simple();
        if simple.is_none() {
            return;
        }
        let mut key = FindString::default();
        simple.unwrap().visit_children_with(&mut key);

        if key.str.len() != 5 || key.str == "event" || key.str == "kuMu1" {
            return;
        }

        if right_lit.is_some() {
            let mut str = FindString::default();
            right_lit.unwrap().visit_children_with(&mut str);

            if str.str.len() == 0 {
                return;
            }

            self.assignments
                .push(Proxy::string(key.str.to_string(), str.str));
            n.take();
        } else if right_fun.is_some() {
            let fun = right_fun.unwrap();
            // println!("visit_assign_expr: {} -> {:?}", key.str, fun);
            let func = &fun.function;
            let stmts = <Option<swc_ecma_ast::BlockStmt> as Clone>::clone(&func.body)
                .unwrap()
                .stmts;

            let first = stmts.first();

            if first.is_none() {
                return;
            }

            let as_return_stmt = first.unwrap().as_return_stmt();
            if as_return_stmt.is_none() {
                return;
            }
            let mut expr =
                <Option<Box<swc_ecma_ast::Expr>> as Clone>::clone(&as_return_stmt.unwrap().arg)
                    .unwrap();
            // println!("visit_key_value_prop: Unsupported {} (function)", key,);
            let seq = expr.as_seq();
            if seq.is_some() {
                let seq = seq.unwrap();
                expr = seq.exprs.last().unwrap().to_owned();
            }
            let as_call = expr.as_call();
            let as_bin = expr.as_bin();
            if as_call.is_some() {
                // Handle special case:
                // e.pHFEm = function(i, j) {
                //     return i + j;
                // }
                // f.abCxY = function(b, c) {
                //   return e.PHFEm(b, c);
                // }
                // In this case, we need to handle the call as a binary operation
                // Check if the function is a call to another function
                let call = as_call.unwrap();
                let callee = call.callee.as_expr().unwrap();
                if let Some(as_member) = callee.as_member() {
                    let comp = as_member.prop.as_computed().unwrap();
                    let proxy_key = match comp.expr.as_lit().unwrap() {
                        Lit::Str(Str { value, .. }) => Some(value.to_string()),
                        _ => None, // Not a string literal
                    };

                    //Find Proxy with proxy_key in assignments
                    let proxy_key_str = proxy_key.clone().unwrap();
                    let maybe_p = self.assignments.iter().find(|p| p.key == proxy_key_str);
                    if maybe_p.is_none() {
                        return;
                    }
                    let p = maybe_p.unwrap();
                    
                    if p.proxy_type == "call" {
                        //check if assignment already exists for the key
                        if self.assignments.iter().find(|p| p.key == key.str).is_some() {
                            println!("Assignment already exists for key: {}", key.str);
                            return;
                        }
                        self.assignments.push(Proxy::call(key.str));
                    } else if p.proxy_type == "binary" {
                        //check if assignment already exists for the key
                        if self.assignments.iter().find(|p| p.key == key.str).is_some() {
                            println!("Assignment already exists for key: {}", key.str);
                            return;
                        }
                        self.assignments
                            .push(Proxy::binary(key.str, p.bin_operator, p.reversed));
                    }
                }
                else {
                    //check if assignment already exists for the key
                    if self.assignments.iter().find(|p| p.key == key.str).is_some() {
                        println!("Assignment already exists for key: {}", key.str);
                        return;
                    }
                    self.assignments.push(Proxy::call(key.str));
                }
                n.take();
            } else if as_bin.is_some() {
                // let bin = as_bin.unwrap();
                // //check if assignment already exists for the key
                // if self.assignments.iter().find(|p| p.key == key.str).is_some() {
                //     println!("Assignment already exists for key: {}", key.str);
                //     return;
                // }
                // let reversed = bin.right.as_ident().unwrap().sym.as_str()
                //     == fun
                //         .function
                //         .params
                //         .first()
                //         .unwrap()
                //         .pat
                //         .as_ident()
                //         .unwrap()
                //         .sym
                //         .as_str();
                // self.assignments
                //     .push(Proxy::binary(key.str.clone(), bin.op, reversed));
                // n.take();
            }
        }
    }
}

#[derive(Default)]
struct ReplaceProxies {
    assignments: Vec<Proxy>,
}

impl VisitMut for ReplaceProxies {
    fn visit_mut_expr(&mut self, n: &mut Expr) {
        n.visit_mut_children_with(self);

        let as_call = n.as_call();
        let as_member = n.as_member();
        if as_member.is_some() {
            let comp = as_member.unwrap().prop.as_computed();
            if comp.is_none() {
                return;
            }

            let mut str = FindString::default();
            let prop_name = comp.unwrap().to_owned();
            prop_name.visit_children_with(&mut str);
            if str.str.len() != 5 {
                return;
            }

            let maybe_p = self.assignments.iter().find(|p| p.key == str.str);

            if maybe_p.is_none() {
                return;
            }
            let p = maybe_p.unwrap();

            if p.proxy_type == "string" {
                // println!("ReplaceProxies: {:?} {}", n, p.string_value);
                *n = Expr::from(Atom::new(p.string_value.to_owned()));
            }
        } else if as_call.is_some() {
            let call = as_call.unwrap();
            let as_member = call.callee.as_expr().unwrap().as_member();
            if as_member.is_none() {
                return;
            }
            let comp = as_member.unwrap().prop.as_computed();
            if comp.is_none() {
                return;
            }

            let mut str = FindString::default();
            let prop_name = comp.unwrap().to_owned();
            prop_name.visit_children_with(&mut str);
            if str.str.len() != 5 {
                return;
            }

            let maybe_p = self.assignments.iter().find(|p| p.key == str.str);

            if maybe_p.is_none() {
                return;
            }
            let p = maybe_p.unwrap();

            let args = &call.args;

            if p.proxy_type == "binary" {
                // println!(
                //     "ReplaceProxies: {}: {} {} {:?}\n",
                //     p.proxy_type, p.key, p.bin_operator, args
                // );
                let left = &args.first().unwrap().expr;
                let right = &args.last().unwrap().expr;
                
                let left_expr = Box::new(Expr::Paren(ParenExpr {
                    span: Span::dummy(),
                    expr: Box::new(*left.to_owned()),
                }));

                let right_expr = Box::new(Expr::Paren(ParenExpr {
                    span: Span::dummy(),
                    expr: Box::new(*right.to_owned()),
                }));

                *n = Expr::Paren(ParenExpr {
                    span: Span::dummy(),
                    expr: Box::new(Expr::from(BinExpr {
                        span: Span::dummy(),
                        op: p.bin_operator,
                        left: if p.reversed { right_expr.clone() } else { left_expr.clone() },
                        right: if p.reversed { left_expr } else { right_expr },
                    })),
                });
            } else if p.proxy_type == "call" {
                let mut vec_args = args.to_vec();
                let callee = vec_args.remove(0);
                *n = Expr::from(CallExpr {
                    span: Span::dummy(),
                    callee: swc_ecma_ast::Callee::Expr(Box::new(*callee.expr.to_owned())),
                    args: vec_args,
                    type_args: None,
                })
            }
        }
    }
}

pub struct Visitor;
impl VisitMut for Visitor {
    fn visit_mut_program(&mut self, program: &mut Program) {
        println!("[*] Finding proxy functions");
        let mut obf_strings = FindProxyAssignments::default();
        program.visit_mut_children_with(&mut obf_strings);

        let mut replacer = ReplaceProxies::default();
        replacer.assignments = obf_strings.assignments;
        println!("[*] Replacing proxy functions");
        program.visit_mut_children_with(&mut replacer);
    }
}
