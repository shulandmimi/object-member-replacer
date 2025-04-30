use rustc_hash::{FxHashMap, FxHashSet};
use swc_common::{Span, Spanned};
use swc_ecma_ast::{
    ComputedPropName, Expr, Ident, KeyValueProp, Lit, MemberExpr, MemberProp, Prop, PropName,
    PropOrSpread,
};
use swc_ecma_visit::{VisitMut, VisitMutWith};

use omm_core::TokenAllocator;

use crate::transformer::TransformContext;

#[derive(Debug)]
pub struct IdentReplacerConfig {
    pub skip_lits: FxHashSet<Span>,
    pub skip_ranges: FxHashSet<Span>,
}

#[derive(Debug)]
pub struct IdentReplacer {
    pub should_replace_ident_list: FxHashMap<String, FxHashSet<Span>>,
    pub ident_map: FxHashMap<String, String>,
    pub allocator: TokenAllocator,
    config: IdentReplacerConfig,
}

impl IdentReplacer {
    pub fn new(set: FxHashMap<String, FxHashSet<Span>>, config: IdentReplacerConfig) -> Self {
        Self {
            should_replace_ident_list: set,
            allocator: TokenAllocator::new(),
            ident_map: FxHashMap::default(),
            config,
        }
    }

    pub fn with_context(mut self, context: &TransformContext) -> Self {
        self.extend_used_ident(context.options.preserve_keywords.iter().cloned().collect());
        self
    }

    pub fn extend_used_ident(&mut self, set: FxHashSet<String>) {
        self.allocator.extends(set);
    }

    pub fn contain(&self, ident: &str, span: Span) -> bool {
        if self.config.skip_lits.contains(&span) {
            return false;
        }

        self.should_replace_ident_list
            .get(ident)
            .is_some_and(|spans| spans.contains(&span))
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

    fn replace_computed(&mut self, computed_props_name: &mut ComputedPropName) -> bool {
        if let Expr::Lit(Lit::Str(lit)) = &*computed_props_name.expr {
            let v = lit.value.as_str();
            if self.contain(v, lit.span) {
                *computed_props_name = self.create_computed_prop_name(v);
                return true;
            }
        }

        false
    }
}

impl VisitMut for IdentReplacer {
    fn visit_mut_call_expr(&mut self, node: &mut swc_ecma_ast::CallExpr) {
        if let (Some(first), Some(last)) = (node.args.first(), node.args.last()) {
            if self.config.skip_ranges.contains(&Span {
                lo: first.span_lo(),
                hi: last.span_hi(),
            }) {
                return;
            }
        }

        node.visit_mut_children_with(self);
    }

    fn visit_mut_member_expr(&mut self, node: &mut MemberExpr) {
        match &mut node.obj {
            box Expr::Ident(_) => {}
            _ => {
                node.obj.visit_mut_with(self);
            }
        }

        let mut is_replaced = false;
        match &mut node.prop {
            MemberProp::Ident(ident) => {
                let v = ident.sym.as_str();
                if self.contain(v, ident.span) {
                    node.prop = MemberProp::Computed(self.create_computed_prop_name(v));
                    is_replaced = true;
                }
            }

            MemberProp::PrivateName(_) => {}

            MemberProp::Computed(computed_prop_name) => {
                is_replaced = self.replace_computed(computed_prop_name);
            }
        }

        if !is_replaced {
            node.visit_mut_children_with(self);
        }
    }

    fn visit_mut_prop_or_spread(&mut self, node: &mut PropOrSpread) {
        match node {
            PropOrSpread::Prop(box prop) => match prop {
                Prop::Shorthand(v) => {
                    let name = v.sym.as_str();
                    if self.contain(name, v.span) {
                        *prop = Prop::KeyValue(KeyValueProp {
                            key: PropName::Computed(self.create_computed_prop_name(name)),
                            value: Box::new(Expr::Ident(v.clone())),
                        });
                    }
                }
                Prop::KeyValue(v) => {
                    if let PropName::Ident(ident) = &v.key {
                        let name = ident.sym.as_str();
                        if self.contain(name, ident.span) {
                            v.key = PropName::Computed(self.create_computed_prop_name(name));
                        }
                    }
                }

                Prop::Method(v) => {
                    if let PropName::Ident(ident) = &v.key {
                        let name = ident.sym.as_str();
                        if self.contain(name, ident.span) {
                            v.key = PropName::Computed(self.create_computed_prop_name(name));
                        }
                    }
                }
                _ => {}
            },
            PropOrSpread::Spread(_) => {}
        }

        node.visit_mut_children_with(self);
    }

    fn visit_mut_expr(&mut self, node: &mut Expr) {
        if let Expr::Lit(Lit::Str(lit)) = node {
            let v = lit.value.as_str();
            if self.contain(v, lit.span) {
                *node = Expr::Ident(self.create_ident(v));
            }
        }

        node.visit_mut_children_with(self);
    }

    fn visit_mut_prop_name(&mut self, node: &mut PropName) {
        let mut is_replaced = false;
        match node {
            PropName::Ident(ident) => {
                let v = ident.sym.as_str();
                if self.contain(v, ident.span) {
                    *node = PropName::Computed(self.create_computed_prop_name(v));
                    is_replaced = true;
                }
            }
            PropName::Str(str) => {
                let v = str.value.as_str();
                if self.contain(v, str.span) {
                    *node = PropName::Computed(self.create_computed_prop_name(v));
                    is_replaced = true;
                }
            }
            PropName::Computed(computed_prop_name) => {
                is_replaced = self.replace_computed(computed_prop_name);
            }
            _ => {}
        }

        if !is_replaced {
            node.visit_mut_children_with(self);
        }
    }

    fn visit_mut_prop(&mut self, node: &mut Prop) {
        let mut is_replaced = false;
        match node {
            Prop::Shorthand(ident) => {
                let name = ident.sym.as_str();
                if self.contain(name, ident.span) {
                    *node = Prop::KeyValue(KeyValueProp {
                        key: PropName::Computed(self.create_computed_prop_name(name)),
                        value: Box::new(Expr::Ident(ident.clone())),
                    });
                    is_replaced = true;
                }
            }
            _ => {
                node.visit_mut_children_with(self);
            }
        }

        if !is_replaced {
            node.visit_mut_children_with(self);
        }
    }
}
