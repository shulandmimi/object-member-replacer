use std::{cell::RefCell, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};

use swc_common::{util::take::Take, Mark, Span};
use swc_ecma_ast::{Expr, Id, IdentName, Lit, MemberExpr, MemberProp, Str};
use swc_ecma_utils::ident::IdentLike;
use swc_ecma_visit::{Visit, VisitWith};

use crate::transformer::TransformContext;

pub type IdentCollectorData = FxHashMap<String, (FxHashSet<Span>, usize)>;

#[allow(dead_code)]
#[derive(Debug)]
pub struct IdentCollector {
    pub field: IdentCollectorData,
    // TODO: collect more detailed data, such as variable declarations, parameters, functions, etc.
    pub unresolved_ident: FxHashSet<String>,
    pub top_level_ident: FxHashSet<String>,
    pub used_ident: FxHashSet<String>,
    pub top_level_mark: Mark,
    pub unresolved_mark: Mark,
    ignore_words: FxHashSet<String>,
    string_literal_enable: bool,
    trie: Trie,
    state: MemberMatcherState,
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
            used_ident: Default::default(),
            trie: Default::default(),
            state: MemberMatcherState::default(),
        }
    }

    fn count_str(&mut self, ident: &str, span: Span) {
        let (spans, count) = self
            .field
            .entry(ident.to_string())
            .or_insert_with(|| (FxHashSet::default(), 0));

        spans.insert(span);
        *count += 1;
    }

    fn count_lit(&mut self, ident: &Str) {
        self.count_str(&ident.value, ident.span);
    }

    fn count(&mut self, ident: &IdentName) {
        self.count_str(&ident.sym, ident.span);
    }

    pub fn with_context(mut self, context: &TransformContext) -> Self {
        // self.ignore_words = context.options.ignore_words.iter().cloned().collect();
        self.trie = Trie::from(context.options.ignore_words.to_vec());
        self.string_literal_enable = context.options.string_literal;
        self
    }

    fn with_state<F: FnOnce(&mut Self)>(&mut self, state: MemberMatcherState, f: F) {
        let prev = self.state;
        self.state = state;
        f(self);
        self.state = prev;
    }
}

impl Visit for IdentCollector {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        let mut is_matched = false;

        if matches!(self.state, MemberMatcherState::Visitor) {
            let mut matcher = MemberMatcher::new(&self.trie);

            node.visit_with(&mut matcher);

            let MemberMatcherResult {
                is_matched: matched,
                ..
            } = matcher.take_result();

            is_matched = matched;
        }

        if is_matched {
            self.with_state(MemberMatcherState::Matche, |this| {
                node.visit_with(this);
            });
            return;
        }

        {
            let is_match_mode = matches!(self.state, MemberMatcherState::Matche);
            match &node.obj {
                box Expr::Member(member) => {
                    member.visit_with(self);
                }
                box Expr::Ident(_) => {}
                _ => {
                    self.with_state(MemberMatcherState::Visitor, |this| {
                        node.obj.visit_with(this);
                    });
                }
            }

            match &node.prop {
                MemberProp::Ident(ident_name) => {
                    if !is_match_mode {
                        self.count(ident_name);
                    }
                }
                MemberProp::PrivateName(_) => {}
                MemberProp::Computed(computed_prop_name) => {
                    if !is_match_mode && let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                        self.count_lit(lit);
                        return;
                    }
                    self.with_state(MemberMatcherState::Visitor, |this| {
                        computed_prop_name.visit_with(this);
                    });
                }
            }
        }
    }

    fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident) {
        self.used_ident.insert(ident.sym.to_string());
    }

    fn visit_lit(&mut self, lit: &Lit) {
        if self.string_literal_enable
            && let Lit::Str(lit) = lit
        {
            self.count_lit(lit);
        } else {
            lit.visit_children_with(self);
        }
    }
}

#[derive(Debug, Default)]
struct TrieNode {
    children: FxHashMap<Rc<String>, Rc<RefCell<TrieNode>>>,
    key: Rc<String>,
    mark: bool,
}

#[derive(Debug, Default)]
struct Trie {
    root: Rc<RefCell<TrieNode>>,
}

impl Trie {
    fn insert(&mut self, value: String) {
        let mut current = self.root.clone();
        for ch in value.split('.') {
            let key = Rc::new(ch.to_string());
            let mut current_ref = current.borrow_mut();

            if !current_ref.children.contains_key(&key) {
                current_ref.children.insert(
                    key.clone(),
                    Rc::new(RefCell::new(TrieNode {
                        children: Default::default(),
                        key: key.clone(),
                        mark: false,
                    })),
                );
            }

            let v = current_ref.children[&key].clone();

            drop(current_ref);

            current = v;
        }

        current.borrow_mut().mark = true;
    }

    fn query(&self, value: String) -> bool {
        let mut current = self.root.clone();
        let mut near_mark = false;
        for ch in value.split('.') {
            let key = Rc::new(ch.to_string());
            let current_ref = current.borrow();

            near_mark = current_ref.mark;

            if !current_ref.children.contains_key(&key) {
                return near_mark;
            }

            let v = current_ref.children.get(&key).unwrap().clone();
            drop(current_ref);

            current = v;
        }

        let v = current.borrow();

        v.mark || near_mark
    }
}

impl From<Vec<String>> for Trie {
    fn from(value: Vec<String>) -> Self {
        let mut root = Trie::default();

        for item in value {
            root.insert(item);
        }

        root
    }
}

struct MemberMatcherResult {
    is_matched: bool,
    idents: FxHashMap<String, Span>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
enum MemberMatcherState {
    Matche,
    #[default]
    Visitor,
}
#[derive(Debug)]
struct MemberMatcher<'a> {
    pub field: &'a Trie,
    pub paths: Vec<(String, Id, Span)>,
    pub state: MemberMatcherState,
    pub idents: Vec<(String, Id, Span)>,
    pub matched: bool,
}

impl<'a> MemberMatcher<'a> {
    fn new(trie: &'a Trie) -> Self {
        Self {
            field: trie,
            paths: Default::default(),
            state: Default::default(),
            idents: Default::default(),
            matched: false,
        }
    }

    fn with_state<F: FnOnce(&mut Self)>(&mut self, state: MemberMatcherState, f: F) {
        let prev = self.state;
        let prev_data = if matches!(prev, MemberMatcherState::Matche) && prev != state {
            Some(self.paths.take())
        } else {
            None
        };
        self.state = state;
        f(self);
        self.state = prev;
        if let Some(data) = prev_data {
            self.paths = data;
        }
    }

    fn take_result(self) -> MemberMatcherResult {
        MemberMatcherResult {
            is_matched: self.matched,
            idents: self
                .idents
                .into_iter()
                .map(|(ident, _, span)| (ident, span))
                .collect(),
        }
    }

    // fn try_match_member_expr<'a>(&mut self, node: &'a mut MemberExpr) -> TryMatchMemberExprResult<'a> {

    //     // TryMatchMemberExprResult {
    //     //     is_matched: is_end && is_ident_chain,
    //     //     walkable_items: result,
    //     // }
    // }
}

impl Visit for MemberMatcher<'_> {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        self.with_state(MemberMatcherState::Matche, |this| {
            let mut is_end = false;
            let mut is_ident_chain = false;

            match &node.prop {
                MemberProp::Ident(ident) => {
                    this.paths
                        .push((ident.sym.to_string(), ident.sym.to_id(), ident.span));
                }
                MemberProp::PrivateName(name) => {
                    this.paths
                        .push((name.name.to_string(), name.name.to_id(), name.span));
                }
                MemberProp::Computed(computed_prop_name) => {
                    if let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                        this.paths
                            .push((lit.value.to_string(), lit.value.to_id(), lit.span));
                    }
                }
            }

            match &node.obj {
                box Expr::Ident(ident) => {
                    this.paths
                        .push((ident.sym.to_string(), ident.sym.to_id(), ident.span));
                    is_end = true;
                    is_ident_chain = true;
                }
                box Expr::Member(member) => {
                    member.visit_with(this);
                }
                _ => {
                    // this.with_state(MemberMatcherState::Visitor, |this| {
                    //     node.obj.visit_mut_with(this);
                    // });
                    is_end = true;
                    is_ident_chain = false;
                }
            }

            if is_end {
                let is_matched = if is_ident_chain {
                    this.field.query(
                        this.paths
                            .iter()
                            .map(|(v, _, _)| v)
                            .cloned()
                            .rev()
                            .collect::<Vec<_>>()
                            .join("."),
                    )
                } else {
                    false
                };

                this.matched = is_matched;

                if !is_matched {
                    let mut paths = this.paths.take();
                    paths.pop();
                    paths.reverse();
                    this.idents.extend(paths)
                }

                this.paths.clear();
            }
        });
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use std::sync::Arc;
    use swc_common::Globals;

    use swc_ecma_parser::{EsSyntax, Syntax};

    use crate::{parse, util::resolve_module_mark, ModuleType, TransformOption};

    use super::*;

    #[test]
    fn t1() -> Result<()> {
        let mut v = parse(
            Arc::new(
                r#"
__target__;
process.env.NODE_ENV;
process.env.NODE_ENV.aaa
process.env;
a.b.c.d.e.f;
a.b.c;
a.b.c[process.env.NODE_ENV];
a(a.b.c).b.c;
a.c.d
"#
                .to_string(),
            ),
            Syntax::Es(EsSyntax::default()),
        )?;

        let globals = Globals::default();

        let (unresolved_mark, top_level_mark) = resolve_module_mark(&mut v, false, &globals);

        let context = TransformContext {
            module_type: ModuleType::Javascript,
            options: TransformOption {
                ignore_words: vec![
                    "process.env.NODE_ENV".to_string(),
                    "__target__".to_string(),
                    "a.b.c".to_string(),
                    "a.b".to_string(),
                ],
                ..Default::default()
            },
            globals: Arc::new(globals),
        };

        let mut collector =
            IdentCollector::new(unresolved_mark, top_level_mark).with_context(&context);

        // let mut matcher = MemberMatcher::new(&trie);
        v.visit_with(&mut collector);

        println!("{:#?}", collector);

        // v.visit_with(&mut matcher);

        // println!("{:#?}", matcher.idents);

        Ok(())
    }
}
