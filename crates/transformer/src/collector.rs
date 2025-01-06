use rustc_hash::{FxHashMap, FxHashSet};

use swc_common::Mark;
use swc_ecma_ast::{Expr, IdentName, Lit, MemberExpr, MemberProp};
use swc_ecma_visit::{Visit, VisitWith};

use crate::transformer::TransformContext;

#[allow(dead_code)]
#[derive(Debug)]
pub struct IdentCollector {
    pub field: FxHashMap<String, usize>,
    // TODO: collect more detailed data, such as variable declarations, parameters, functions, etc.
    pub unresolved_ident: FxHashSet<String>,
    pub top_level_ident: FxHashSet<String>,
    pub used_ident: FxHashSet<String>,
    pub top_level_mark: Mark,
    pub unresolved_mark: Mark,
    ignore_words: FxHashSet<String>,
    string_literal_enable: bool,
}

impl IdentCollector {
    pub fn new(top_level_mark: Mark, unresolved_mark: Mark) -> Self {
        Self {
            field: Default::default(),
            unresolved_ident: Default::default(),
            top_level_ident: Default::default(),
            top_level_mark,
            unresolved_mark,
            ignore_words: Default::default(),
            string_literal_enable: Default::default(),
            used_ident: Default::default()
        }
    }

    fn count_str(&mut self, ident: &str) {
        let count = self.field.entry(ident.to_string()).or_insert(0);
        *count += 1;
    }

    fn count(&mut self, ident: &IdentName) {
        let name = ident.sym.as_str();
        self.count_str(name);
    }

    pub fn with_context(mut self, context: &TransformContext) -> Self {
        self.ignore_words = context.options.ignore_words.iter().cloned().collect();
        self.string_literal_enable = context.options.string_literal;
        self
    }
}

impl Visit for IdentCollector {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        if match &node.obj {
            box Expr::Ident(ref ident) => self.ignore_words.contains(ident.sym.as_str()),
            _ => false,
        } {
            node.visit_children_with(self);
            return
        }

        match &node.prop {
            MemberProp::Ident(ident_name) => {
                self.count(ident_name);
            }
            MemberProp::PrivateName(name) => {
                name.visit_with(self);
            }
            MemberProp::Computed(computed_prop_name) => {
                if let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                    self.count_str(lit.value.as_str());
                    return;
                }
                computed_prop_name.visit_with(self);
            }
        }
    }

    fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident) {
        self.used_ident.insert(ident.sym.to_string());
    }

    fn visit_lit(&mut self, lit: &Lit) {
        if self.string_literal_enable && let Lit::Str(lit) = lit {
            self.count_str(lit.value.as_str());
        } else {
            lit.visit_children_with(self);
        }
    }
}
