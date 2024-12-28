use std::collections::HashMap;

use swc_ecma_ast::{Expr, IdentName, Lit, MemberExpr, MemberProp};
use swc_ecma_visit::Visit;


#[derive(Debug, Default)]
pub struct IdentCollector {
    pub field: HashMap<String, usize>,
}

impl IdentCollector {
    pub fn new() -> Self {
        Default::default()
    }

    fn count_str(&mut self, ident: &str) {
        let count = self.field.entry(ident.to_string()).or_insert(0);
        *count += 1;
    }

    fn count(&mut self, ident: &IdentName) {
        let name = ident.sym.as_str();
        self.count_str(name);
    }
}

impl Visit for IdentCollector {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        match &node.prop {
            MemberProp::Ident(ident_name) => {
                self.count(ident_name);
            }
            MemberProp::PrivateName(_) => {}
            MemberProp::Computed(computed_prop_name) => {
                if let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                    self.count_str(lit.value.as_str());
                }
            }
        }
    }
}
