use regex::Regex;
use swc_core::ecma::atoms::JsWord;
use swc_core::ecma::visit::{VisitMut, VisitMutWith};
use swc_ecma_ast::{Expr, Lit, Program, Str, AssignOp, FnExpr, FnDecl};
use swc_ecma_visit::{Visit, VisitWith};
use swc_common::util::take::Take;

#[derive(Default)]
struct FindInteger {
    ints: Vec<f64>,
}
impl Visit for FindInteger {
    fn visit_number(&mut self, n: &swc_ecma_ast::Number) {
        self.ints.push(n.value)
    }
}

#[derive(Default)]

struct ReplaceProxyCalls {
    subtract: i32,
    strings: Vec<String>,
    assignments: Vec<String> // all variables that points to function b
}

impl ReplaceProxyCalls {
    pub fn new(subtract: i32, strings: Vec<String>) -> Self {
        Self { subtract, strings, assignments: vec![] }
    }
}

impl VisitMut for ReplaceProxyCalls {
    fn visit_mut_expr(&mut self, expr: &mut swc_ecma_ast::Expr) {
        expr.visit_mut_children_with(self);

        if !expr.is_call() {
            return;
        }
        let n = expr.as_call().unwrap();

        if n.args.len() != 1 {
            return;
        }
        //check if callee is in assignments
        let callee = n.callee.as_expr().unwrap().as_ident();
        if let Some(callee) = callee {
            if !self.assignments.contains(&callee.sym.to_string()) {
                return;
            }
        }

        let arg = n.args[0].expr.as_lit();
        if let Some(p) = arg {
            let mut find = FindInteger::default();
            p.to_owned().visit_children_with(&mut find);
            if find.ints.len() == 1 {
                let i: i32 = find.ints[0] as i32;

                let works = usize::try_from(i - self.subtract);
                if let Ok(res) = works {
                    if self.strings.len() > res {
                        let str = self.strings[res].to_owned();
                        *expr = Expr::Lit(Lit::Str(Str::from(str)));
                    }
                }
            }
        }
    }

    fn visit_mut_fn_decl(&mut self, n: &mut FnDecl) {
        let params_in_assignments = n.function.params.iter().filter(|x| self.assignments.contains(&x.pat.as_ident().unwrap().sym.to_string())).map(|x| x.pat.as_ident().unwrap().sym.to_string()).collect::<Vec<String>>();
        let params_not_in_assignments = n.function.params.iter().filter(|x| !self.assignments.contains(&x.pat.as_ident().unwrap().sym.to_string())).map(|x| x.pat.as_ident().unwrap().sym.to_string()).collect::<Vec<String>>();
        // // Remove identifiers from assignments that are in the function params
        self.assignments = self.assignments.iter().filter(|x| !params_in_assignments.contains(x)).map(|x| x.to_string()).collect();
        n.visit_mut_children_with(self);
        // // Restore the assignments
        self.assignments = self.assignments.iter().filter(|x| !params_not_in_assignments.contains(x)).map(|x| x.to_string()).collect();
        // // Add assignments that were already in the function params
        self.assignments.extend(params_in_assignments);
    }

    fn visit_mut_fn_expr(&mut self, n: &mut FnExpr) {
        let params_in_assignments = n.function.params.iter().filter(|x| self.assignments.contains(&x.pat.as_ident().unwrap().sym.to_string())).map(|x| x.pat.as_ident().unwrap().sym.to_string()).collect::<Vec<String>>();
        let params_not_in_assignments = n.function.params.iter().filter(|x| !self.assignments.contains(&x.pat.as_ident().unwrap().sym.to_string())).map(|x| x.pat.as_ident().unwrap().sym.to_string()).collect::<Vec<String>>();
        // // Remove identifiers from assignments that are in the function params
        self.assignments = self.assignments.iter().filter(|x| !params_in_assignments.contains(x)).map(|x| x.to_string()).collect();
        n.visit_mut_children_with(self);
        // // Restore the assignments
        self.assignments = self.assignments.iter().filter(|x| !params_not_in_assignments.contains(x)).map(|x| x.to_string()).collect();
        // // Add assignments that were already in the function params
        self.assignments.extend(params_in_assignments);
    }

    fn visit_mut_assign_expr(&mut self, n: &mut swc_ecma_ast::AssignExpr) {
        n.visit_mut_children_with(self);
        if n.op != AssignOp::Assign {
            return;
        }
        let right_var = n.right.as_ident();
        // Check if expression is xx = b
        if let Some(right) = right_var {
            let left = n.left.as_ident();
            if let Some(left) = left {
                if right.sym == "b" {
                    self.assignments.push(left.sym.to_string());
                    n.take();
                }
                else if self.assignments.contains(&right.sym.to_string()) {
                    self.assignments.push(left.sym.to_string());
                    n.take();
                }
            }
        }
    }
}

#[derive(Default)]
struct FindAllStrings {
    done_string: bool,
    done_json: bool,
    strings: Vec<String>,
    json_start: u32,
}

impl VisitMut for FindAllStrings {
    fn visit_mut_atom(&mut self, n: &mut JsWord) {
        if self.done_string {
            return;
        }
        let length = n.len();

        if length > 200 {
            let re = Regex::new(r"bigint(?<delimiter>.)").unwrap();
            let all: String = n.to_string();
            if let Some(caps) = re.captures(&all) {
                self.done_string = true;
                let delimiter = &caps["delimiter"];
                self.strings = all.split(delimiter).map(String::from).collect();
            }
        }
    }
    fn visit_mut_ident(&mut self, n: &mut swc_ecma_ast::Ident) {
        if n.sym != "JSON" || self.done_json {
            return;
        }
        self.json_start = n.span.lo.0 + 6;
        self.done_json = true;
    }
}

#[derive(Default)]
struct RemoveBigString;

impl VisitMut for RemoveBigString {
    fn visit_mut_atom(&mut self, n: &mut JsWord) {
        let length = n.len();

        if length > 200 {
            *n = JsWord::new("\"\"");
        }
    }
}

pub struct Visitor {
    source: String,
    stringify: i32,
    subtract: i32,
}

impl Visitor {
    pub fn new(source: String) -> Self {
        Self {
            source,
            stringify: 0,
            subtract: 0,
        }
    }
}

impl VisitMut for Visitor {
    fn visit_mut_program(&mut self, program: &mut Program) {
        println!("[*] Finding string array");
        let mut obf_strings = FindAllStrings::default();
        program.visit_mut_children_with(&mut obf_strings);
        if !obf_strings.done_string || !obf_strings.done_json {
            println!("[!] Error finding string array");
            return;
        }
        let splits = self
            .source
            .split_at(obf_strings.json_start.try_into().unwrap())
            .1
            .split_at(20)
            .0;

        let first_int_re = Regex::new(r"(?<int>\d+)").unwrap();
        if let Some(caps) = first_int_re.captures(splits) {
            self.stringify = caps["int"].parse::<i32>().unwrap();
        }
        let subtract_re = Regex::new(r".=.-(?<subtract>\d+?),.=").unwrap();
        if let Some(caps) = subtract_re.captures(&self.source) {
            self.subtract = caps["subtract"].parse::<i32>().unwrap();
        }

        program.visit_mut_children_with(&mut RemoveBigString::default());

        // println!("stringify: {}, subtract: {}", self.stringify, self.subtract);
        loop {
            obf_strings.strings.rotate_left(1);
            if obf_strings.strings[usize::try_from(self.stringify - self.subtract).unwrap()]
                == "stringify"
            {
                break;
            }
        }

        program.visit_mut_children_with(&mut ReplaceProxyCalls::new(
            self.subtract,
            obf_strings.strings,
        ));
    }
}
