use swc_ecma_ast::{
    ComputedPropName, Expr, Ident, KeyValueProp, Lit, MemberExpr, MemberProp, Prop, PropName,
    PropOrSpread,
};
use swc_ecma_visit::VisitMut;

use std::collections::{HashMap, HashSet};

use omm_core::TokenAllocator;

#[derive(Debug)]
pub struct IdentReplacer {
    pub should_replace_ident_list: HashSet<String>,
    pub ident_map: HashMap<String, String>,
    pub allocator: TokenAllocator,
}

impl IdentReplacer {
    pub fn new(set: HashSet<String>) -> Self {
        Self {
            should_replace_ident_list: set,
            allocator: TokenAllocator::new(),
            ident_map: HashMap::new(),
        }
    }

    pub fn contain(&self, ident: &str) -> bool {
        self.should_replace_ident_list.contains(ident)
    }

    pub fn alloc_ident(&mut self, ident: &str) -> String {
        if let Some(v) = self.ident_map.get(ident) {
            return v.to_string();
        }

        let s = self.allocator.alloc();

        self.ident_map.insert(ident.to_string(), s.clone());

        s
    }
}

impl From<HashMap<String, usize>> for IdentReplacer {
    fn from(value: HashMap<String, usize>) -> Self {
        Self::new(value.into_keys().collect())
    }
}

impl IdentReplacer {
    fn create_ident(&mut self, ident: &str) -> Ident {
        let s = self.alloc_ident(ident);

        Ident {
            sym: s.into(),
            span: Default::default(),
            optional: false,
            ctxt: Default::default(),
        }
    }

    fn create_computed_prop_name(&mut self, name: &str) -> ComputedPropName {
        ComputedPropName {
            span: Default::default(),
            expr: Box::new(Expr::Ident(self.create_ident(name))),
        }
    }

    fn replace_computed(&mut self, computed_props_name: &mut ComputedPropName) {
        if let Expr::Lit(Lit::Str(lit)) = &*computed_props_name.expr {
            let v = lit.value.as_str();
            if self.contain(v) {
                *computed_props_name = self.create_computed_prop_name(v);
            }
        }
    }
}

impl VisitMut for IdentReplacer {
    fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
        match &mut node.prop {
            MemberProp::Ident(ident) => {
                let v = ident.sym.as_str();
                if self.contain(v) {
                    node.prop = MemberProp::Computed(self.create_computed_prop_name(v));
                }
            }
            MemberProp::PrivateName(_) => {}
            MemberProp::Computed(computed_prop_name) => {
                self.replace_computed(computed_prop_name);
            }
        }
    }

    fn visit_mut_prop_or_spread(&mut self, node: &mut PropOrSpread) {
        match node {
            PropOrSpread::Prop(box prop) => match prop {
                Prop::Shorthand(v) => {
                    let name = v.sym.as_str();
                    if self.contain(name) {
                        *prop = Prop::KeyValue(KeyValueProp {
                            key: PropName::Computed(self.create_computed_prop_name(name)),
                            value: Box::new(Expr::Ident(v.clone())),
                        });
                    }
                }
                Prop::KeyValue(v) => {
                    if let PropName::Ident(ident) = &v.key {
                        let name = ident.sym.as_str();
                        if self.contain(name) {
                            v.key = PropName::Computed(self.create_computed_prop_name(name));
                        }
                    }
                }

                Prop::Method(v) => {
                    if let PropName::Ident(ident) = &v.key {
                        let name = ident.sym.as_str();
                        if self.contain(name) {
                            v.key = PropName::Computed(self.create_computed_prop_name(name));
                        }
                    }
                }
                _ => {}
            },
            PropOrSpread::Spread(_) => {}
        }
    }
}
