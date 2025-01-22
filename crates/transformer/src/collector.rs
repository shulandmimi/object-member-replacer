use std::{cell::RefCell, fmt::Debug, mem, rc::Rc};

use rustc_hash::{FxHashMap, FxHashSet};

use swc_common::{util::take::Take, Mark, Span};
use swc_ecma_ast::{
    CallExpr, Callee, Expr, ExprOrSpread, Ident, IdentName, Lit, MemberExpr, MemberProp, Prop,
    PropName, Str,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::transformer::{IgnoreWord, MemberMatchOption, StringLitOptions, TransformContext};

pub type IdentCollectorData = FxHashMap<String, (FxHashSet<Span>, usize)>;
type ContainMemberMatch = Rc<Vec<MemberMatchOption>>;
type IgnoreWordTrieValue = (usize, IgnoreWord);

#[allow(dead_code)]
#[derive(Debug)]
pub struct IdentCollector {
    pub field: IdentCollectorData,
    pub skip_lits: FxHashSet<Span>,
    // TODO: collect more detailed data, such as variable declarations, parameters, functions, etc.
    pub unresolved_ident: FxHashSet<String>,
    pub top_level_ident: FxHashSet<String>,
    pub used_ident: FxHashSet<String>,
    pub top_level_mark: Mark,
    pub unresolved_mark: Mark,
    trie: Trie<(usize, IgnoreWord)>,
    state: CollectorMemberMatcherState,
    pending_store_arg_lits: FxHashSet<(Str, Span)>,
    contain_member_match_list: ContainMemberMatch,
    skip_strings: FxHashSet<String>,
}

impl IdentCollector {
    pub fn new(top_level_mark: Mark, unresolved_mark: Mark) -> Self {
        Self {
            field: Default::default(),
            unresolved_ident: Default::default(),
            top_level_ident: Default::default(),
            top_level_mark,
            unresolved_mark,
            used_ident: Default::default(),
            trie: Trie::new(),
            state: CollectorMemberMatcherState::default(),
            skip_lits: Default::default(),
            pending_store_arg_lits: Default::default(),
            contain_member_match_list: Default::default(),
            skip_strings: FxHashSet::default(),
        }
    }

    fn count_str(&mut self, ident: &str, span: Span) {
        if self.skip_lits.contains(&span) {
            return;
        }

        let (spans, count) = self
            .field
            .entry(ident.to_string())
            .or_insert_with(|| (FxHashSet::default(), 0));

        spans.insert(span);
        *count += 1;
    }

    fn count_lit(&mut self, ident: &Str) {
        if self.skip_strings.contains(ident.value.as_ref()) {
            return;
        }

        self.count_str(&ident.value, ident.span);
    }

    fn count_ident(&mut self, ident: &Ident) {
        self.count_str(&ident.sym, ident.span);
    }

    fn count_ident_name(&mut self, ident: &IdentName) {
        self.count_str(&ident.sym, ident.span);
    }

    pub fn with_context(mut self, context: &TransformContext) -> Self {
        for (index, item) in context.options.ignore_words.iter().enumerate() {
            if let Some(path) = item.path() {
                self.trie
                    .insert(path.to_string(), Some((index, item.clone())));
            }

            if let IgnoreWord::StringLit(StringLitOptions { content, .. }) = item {
                self.skip_strings.insert(content.to_string());
            }
        }

        self
    }

    fn with_state<F: FnOnce(&mut Self)>(&mut self, state: CollectorMemberMatcherState, f: F) {
        let prev = self.state;
        self.state = state;
        f(self);
        self.state = prev;
    }

    fn process_matcher_result(&mut self, matcher_result: MemberMatcherResult) -> bool {
        let pending_store_arg_lits = mem::take(&mut self.pending_store_arg_lits);

        let MemberMatcherResult {
            is_matched: matched,
            ident_list,
            match_result,
            skip_spans,
            ..
        } = matcher_result;

        for (ident, span) in ident_list {
            self.count_str(&ident, span);
        }

        if matched {
            self.skip_lits.extend(skip_spans);
        }

        #[allow(clippy::collapsible_match)]
        if let Some((_, options)) = match_result {
            if let Some((_, options)) = options.as_deref() {
                if options.skip_lit_arg() {
                    self.skip_lits.extend(
                        pending_store_arg_lits
                            .into_iter()
                            .map(|(_, span)| span)
                            .collect::<Vec<_>>(),
                    );
                } else {
                    pending_store_arg_lits.into_iter().for_each(|(lit, _)| {
                        self.count_lit(&lit);
                    });
                }
            }
        }

        matched
    }
}

impl Visit for IdentCollector {
    fn visit_call_expr(&mut self, node: &CallExpr) {
        if matches!(self.state, CollectorMemberMatcherState::Visitor)
            && let Callee::Expr(ref expr) = node.callee
            && matches!(expr, box Expr::Member(_) | box Expr::Ident(_))
        {
            node.args.iter().for_each(|arg| {
                #[allow(clippy::collapsible_match)]
                #[allow(irrefutable_let_patterns)]
                if let ExprOrSpread { box expr, .. } = arg {
                    if let Expr::Lit(Lit::Str(ref lit)) = expr {
                        self.pending_store_arg_lits.insert((lit.clone(), lit.span));
                    }
                }
            });
        }
        node.visit_children_with(self);
    }

    fn visit_member_expr(&mut self, node: &MemberExpr) {
        let mut is_matched = false;

        if matches!(self.state, CollectorMemberMatcherState::Visitor) {
            let mut matcher =
                MemberMatcher::new(&self.trie, self.contain_member_match_list.clone());

            node.visit_with(&mut matcher);

            let matched = self.process_matcher_result(matcher.take_result());

            is_matched = matched;
        } else {
            self.pending_store_arg_lits.clear();
        }

        if is_matched {
            self.with_state(CollectorMemberMatcherState::Match, |this| {
                node.visit_with(this);
            });
            return;
        }

        {
            let is_match_mode = matches!(self.state, CollectorMemberMatcherState::Match);
            match &node.obj {
                box Expr::Member(member) => {
                    member.visit_with(self);
                }
                box Expr::Ident(_) => {}
                _ => {
                    self.with_state(CollectorMemberMatcherState::Visitor, |this| {
                        node.obj.visit_with(this);
                    });
                }
            }

            match &node.prop {
                MemberProp::Ident(ident_name) => {
                    if !is_match_mode {
                        self.count_ident_name(ident_name);
                    }
                }
                MemberProp::PrivateName(_) => {}
                MemberProp::Computed(computed_prop_name) => {
                    if !is_match_mode && let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                        self.count_lit(lit);
                        return;
                    }
                    self.with_state(CollectorMemberMatcherState::Visitor, |this| {
                        computed_prop_name.visit_with(this);
                    });
                }
            }
        }
    }

    fn visit_ident(&mut self, ident: &swc_ecma_ast::Ident) {
        if matches!(self.state, CollectorMemberMatcherState::Visitor)
            && !self.pending_store_arg_lits.is_empty()
        {
            let mut matcher: MemberMatcher<'_, IgnoreWordTrieValue> =
                MemberMatcher::new(&self.trie, self.contain_member_match_list.clone());

            ident.visit_with(&mut matcher);

            if self.process_matcher_result(matcher.take_result()) {
                return;
            };
        }

        self.used_ident.insert(ident.sym.to_string());
    }

    fn visit_lit(&mut self, lit: &Lit) {
        if let Lit::Str(lit) = lit {
            self.count_lit(lit);
        } else {
            lit.visit_children_with(self);
        }
    }
}

#[derive(Debug, Default)]
struct TrieNode<T: Debug> {
    children: FxHashMap<Rc<String>, Rc<RefCell<TrieNode<T>>>>,
    mark: bool,
    value: Option<Rc<T>>,
}

#[derive(Debug, Default)]
struct Trie<T: Debug> {
    root: Rc<RefCell<TrieNode<T>>>,
}

impl<T: Debug> Trie<T> {
    fn new() -> Self {
        Self {
            root: Rc::new(RefCell::new(TrieNode {
                children: Default::default(),
                mark: false,
                value: Default::default(),
            })),
        }
    }
    fn insert(&mut self, key: String, value: Option<T>) {
        let mut current = self.root.clone();
        for ch in key.split('.') {
            let key = Rc::new(ch.to_string());
            let mut current_ref = current.borrow_mut();

            if !current_ref.children.contains_key(&key) {
                current_ref.children.insert(
                    key.clone(),
                    Rc::new(RefCell::new(TrieNode {
                        children: Default::default(),
                        mark: false,
                        value: Default::default(),
                    })),
                );
            }

            let v = current_ref.children[&key].clone();

            drop(current_ref);

            current = v;
        }

        current.borrow_mut().mark = true;
        current.borrow_mut().value = value.map(Rc::new);
    }

    fn query(&self, paths: String) -> Option<(usize, Option<Rc<T>>)> {
        let mut current = self.root.clone();
        let mut near_mark = None;
        let mut value = None;
        let keys = paths.split('.').collect::<Vec<_>>();
        let len = keys.len();

        for (index, ch) in keys.into_iter().enumerate() {
            let key = Rc::new(ch.to_string());
            let current_ref = current.borrow();

            let Some(next) = current_ref.children.get(&key).cloned() else {
                return near_mark.map(|pos| (pos, value));
            };
            drop(current_ref);
            let item = next.borrow();

            if item.mark {
                near_mark = Some(index);
                value = item.value.clone();
            }

            drop(item);

            current = next;
        }

        let v = current.borrow();

        if v.mark {
            Some((len - 1, v.value.clone()))
        } else {
            near_mark.map(|pos| (pos, value.clone()))
        }
    }
}

impl From<Vec<String>> for Trie<String> {
    fn from(value: Vec<String>) -> Self {
        let mut root = Trie::default();

        for item in value {
            root.insert(item.clone(), Some(item));
        }

        root
    }
}

struct MemberMatcherResult {
    is_matched: bool,
    ident_list: Vec<(String, Span)>,
    match_result: Option<(usize, Option<Rc<IgnoreWordTrieValue>>)>,
    skip_spans: FxHashSet<Span>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
enum MemberMatcherState {
    Match,
    #[default]
    Visitor,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
enum CollectorMemberMatcherState {
    Match,
    #[default]
    Visitor,
}

#[derive(Debug)]
struct MemberMatcher<'a, T: Debug> {
    pub trie: &'a Trie<T>,
    pub paths: Vec<(String, Span)>,
    pub state: MemberMatcherState,
    pub ident_list: Vec<(String, Span)>,
    pub matched: bool,
    matchd_result: Option<(usize, Option<Rc<IgnoreWordTrieValue>>)>,
    skip_spans: FxHashSet<Span>,
    #[allow(dead_code)]
    contain_member_match_list: ContainMemberMatch,
}

impl<'a, T: Debug> MemberMatcher<'a, T> {
    fn new(trie: &'a Trie<T>, contain_member_match_list: ContainMemberMatch) -> Self {
        Self {
            trie,
            paths: Default::default(),
            state: Default::default(),
            ident_list: Default::default(),
            matched: false,
            matchd_result: None,
            skip_spans: FxHashSet::default(),
            contain_member_match_list,
        }
    }

    fn with_state<F: FnOnce(&mut Self)>(&mut self, state: MemberMatcherState, f: F) {
        let prev = self.state;
        let prev_data =
            (matches!(prev, MemberMatcherState::Match) && prev != state).then(|| self.paths.take());
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
            ident_list: self.ident_list,
            match_result: self.matchd_result,
            skip_spans: self.skip_spans,
        }
    }

    fn process_match_result(
        &mut self,
        match_result: Option<(usize, Option<Rc<IgnoreWordTrieValue>>)>,
    ) {
        self.matched = match_result.is_some();

        if let Some((pos, options)) = match_result {
            let mut paths = self.paths.take();

            // let first_index = options.as_deref().map(|(index, _)| *index);

            // if !self.contain_member_match_list.is_empty() {
            //     let path_arr = paths
            //         .iter()
            //         .map(|item| item.0.to_string())
            //         .collect::<Vec<_>>();
            // }

            if let Some((_, options)) = options.as_deref() {
                let pos = paths.len().max(1) - 1 - pos;

                if options.subpath() {
                    let (left, right) = paths.split_at_mut(pos);

                    self.skip_spans
                        .extend(right.iter().rev().skip(1).map(|(_, span)| span));

                    self.ident_list.extend(
                        left.iter()
                            .map(|(ident, span)| (ident.to_string(), *span))
                            .collect::<Vec<_>>(),
                    );
                } else {
                    self.skip_spans
                        .extend(paths.into_iter().map(|(_, span)| span));
                }
            }

            self.matchd_result = Some((pos, options));
        }
    }
}

impl Visit for MemberMatcher<'_, IgnoreWordTrieValue> {
    fn visit_member_expr(&mut self, node: &MemberExpr) {
        self.with_state(MemberMatcherState::Match, |this| {
            let mut is_end = false;
            let mut is_ident_chain = false;

            match &node.prop {
                MemberProp::Ident(ident) => {
                    this.paths.push((ident.sym.to_string(), ident.span));
                }
                MemberProp::PrivateName(name) => {
                    this.paths.push((name.name.to_string(), name.span));
                }
                MemberProp::Computed(computed_prop_name) => {
                    if let Expr::Lit(Lit::Str(lit)) = &*computed_prop_name.expr {
                        this.paths.push((lit.value.to_string(), lit.span));
                    }
                }
            }

            match &node.obj {
                box Expr::Ident(ident) => {
                    this.paths.push((ident.sym.to_string(), ident.span));
                    is_end = true;
                    is_ident_chain = true;
                }
                box Expr::Member(member) => {
                    member.visit_with(this);
                }
                _ => {
                    is_end = true;
                    is_ident_chain = false;
                }
            }

            if is_end {
                let match_result = is_ident_chain
                    .then(|| {
                        this.trie.query(
                            this.paths
                                .iter()
                                .map(|(v, _)| v)
                                .cloned()
                                .rev()
                                .collect::<Vec<_>>()
                                .join("."),
                        )
                    })
                    .flatten();

                this.process_match_result(match_result);

                this.paths.clear();
            }
        });
    }

    fn visit_ident(&mut self, node: &Ident) {
        self.paths.push((node.sym.to_string(), node.span));
        let match_result = self.trie.query(node.sym.to_string());

        self.process_match_result(match_result);
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Result;
    use std::sync::Arc;
    use swc_common::Globals;

    use swc_ecma_parser::{EsSyntax, Syntax};

    use crate::{parse, util::resolve_module_mark, MemberMatchOption, ModuleType, TransformOption};

    use super::*;

    fn create_collector(code: &str, options: TransformOption) -> Result<IdentCollector> {
        let mut v = parse(Arc::new(code.to_string()), Syntax::Es(EsSyntax::default()))?;

        let globals = Globals::default();

        let (unresolved_mark, top_level_mark) = resolve_module_mark(&mut v, false, &globals);

        let context = TransformContext {
            module_type: ModuleType::Javascript,
            options,
            globals: Arc::new(globals),
        };

        let mut collector =
            IdentCollector::new(unresolved_mark, top_level_mark).with_context(&context);

        // let mut matcher = MemberMatcher::new(&trie);
        v.visit_with(&mut collector);

        Ok(collector)
    }

    #[test]
    fn t1() -> Result<()> {
        let code = r#"
__target__;
// process.env.NODE_ENV;
process.env.NODE_ENV.aaa
// process.env;
// a.b.c.d.e.f;
// a.b.c;
// a.b.c[process.env.NODE_ENV];
// a(a.b.c).b.c;
// a.c.d
"#;

        let collector = create_collector(
            code,
            TransformOption {
                ignore_words: vec!["process.env.NODE_ENV".into()],
                ..Default::default()
            },
        )?;

        assert!(!collector.field.is_empty());
        assert!(collector.field.contains_key("aaa"));

        Ok(())
    }

    #[test]
    fn member_subpath() -> Result<()> {
        let code = r#"
a.b.c.d
        "#;

        let create_collector_with_subpath = |subpath: bool| {
            create_collector(
                code,
                TransformOption {
                    ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                        path: "a.b.c".into(),
                        subpath,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            )
        };

        let collector = create_collector_with_subpath(true)?;

        assert!(collector.field.contains_key("d"));
        assert!(!collector.field.contains_key("c"));

        let collector = create_collector_with_subpath(false)?;

        assert!(!collector.field.contains_key("d"));
        assert!(!collector.field.contains_key("c"));

        Ok(())
    }

    #[test]
    fn member_function_call() -> Result<()> {
        let code = r#"
a.b.c.d("namespace", "google");
        "#;

        let skip_lit_arg = |args: bool| {
            create_collector(
                code,
                TransformOption {
                    ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                        path: "a.b.c".into(),
                        skip_lit_arg: args,
                        subpath: true,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            )
        };

        let collector = skip_lit_arg(true)?;

        assert!(!collector.skip_lits.is_empty());
        assert!(collector.field.contains_key("d"));
        assert!(!collector.field.contains_key("namespace"));

        let collector = skip_lit_arg(false)?;

        assert!(!collector.skip_lits.is_empty());
        assert!(collector.field.contains_key("namespace"));
        assert!(collector.field.contains_key("google"));

        Ok(())
    }

    #[test]
    fn member_require() -> Result<()> {
        let code = r#"
require.async("./foo.js");
        "#;

        let create_collector_with_subpath = |subpath: bool| {
            create_collector(
                code,
                TransformOption {
                    ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                        path: "require".into(),
                        subpath,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            )
        };

        let collector = create_collector_with_subpath(true)?;

        assert!(collector.skip_lits.is_empty());
        assert!(collector.field.contains_key("./foo.js"));
        assert!(collector.field.contains_key("async"));

        let collector = create_collector_with_subpath(false)?;

        assert!(!collector.skip_lits.is_empty());
        assert!(collector.field.contains_key("./foo.js"));
        assert!(!collector.field.contains_key("async"));

        Ok(())
    }

    #[test]
    fn only_require() -> Result<()> {
        let code = r#"
require("./foo.js");
        "#;

        let create_collector_with_skip_lit_arg = |args: bool| {
            create_collector(
                code,
                TransformOption {
                    ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                        path: "require".into(),
                        skip_lit_arg: args,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            )
        };

        let collector = create_collector_with_skip_lit_arg(true)?;

        assert!(!collector.skip_lits.is_empty());
        assert!(collector.field.is_empty());

        let collector = create_collector_with_skip_lit_arg(false)?;

        assert!(collector.skip_lits.is_empty());
        assert!(collector.field.contains_key("./foo.js"));

        Ok(())
    }
}
