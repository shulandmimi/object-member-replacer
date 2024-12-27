#![feature(box_patterns)]

use std::collections::{HashMap, HashSet};

use itertools::Itertools;
use swc_ecma_ast::{
    BindingIdent, ComputedPropName, Decl, Expr, Ident, IdentName, KeyValueProp, Lit, MemberExpr,
    MemberProp, ModuleItem, ObjectLit, Pat, Prop, PropName, PropOrSpread, Stmt, VarDecl,
    VarDeclKind, VarDeclarator,
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};
use tracing::trace;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn init_log() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_env("LOG"))
        .init();
}

fn main() {
    init_log();
    trace!("Hello, world!");
    let syntax = Syntax::Es(EsSyntax::default());

    let input = StringInput::new(
        r#"
const obj = {};

obj.fooooooooooooooooooooooooooooooooooooooo = 1;

obj["fooooooooooooooooooooooooooooooooooooooo"] = 1;

console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
console.log(obj.fooooooooooooooooooooooooooooooooooooooo);
"#,
        Default::default(),
        Default::default(),
    );
    let mut parser = Parser::new(syntax, input, None);

    let mut module = match parser.parse_module() {
        Ok(res) => res,
        Err(err) => {
            let msg = err.kind().msg();
            panic!("Error: {}", msg);
        }
    };

    let mut collector = ObjectMemberCollector::default();

    module.visit_with(&mut collector);

    let ObjectMemberCollector { field } = collector;

    let map = process_ident_map(field);

    let mut replacer = ObjectMemberReplacer::new(map.into_keys().collect());
    module.visit_mut_with(&mut replacer);

    let mut decls: Vec<VarDeclarator> = vec![];

    for (val, key) in replacer.ident_map {
        // items.push(ModuleItem::Stmt(Stmt::Decl(())));
        decls.push(VarDeclarator {
            span: Default::default(),
            name: Pat::Ident(BindingIdent {
                id: key.into(),
                type_ann: None,
            }),
            init: Some(Box::new(Expr::Lit(Lit::Str(val.into())))),
            definite: false,
        });
    }

    module.body.insert(
        0,
        ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
            span: Default::default(),
            ctxt: Default::default(),
            kind: VarDeclKind::Var,
            declare: false,
            decls,
        })))),
    );

    println!("{:#?}", module);
}

fn process_ident_map(map: HashMap<String, usize>) -> HashMap<String, usize> {
    let mut v = map
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .sorted_by_key(|(v, _)| v.len())
        .collect::<Vec<_>>();

    let len = v.len();

    let mut is_fined = false;
    let position = v
        .iter()
        .rev()
        .enumerate()
        .rposition(|(index, (ident, count))| {
            is_fined = true;

            if ident.len() <= 2 {
                return false;
            }

            let cost = IdentCost::new(index, ident.len(), *count);

            if !cost.should_compress() {
                return false;
            }

            true
        });

    if is_fined && position.is_none() {
        return HashMap::new();
    }

    if let Some(position) = position {
        v.truncate(len - position + 1);
    }

    v.into_iter().collect()
}

struct IdentCost {
    first_cost: usize,
    more_cost: isize,
    original_cost: isize,
    compress_cost: isize,
}

impl IdentCost {
    fn new(pos: usize, ch_len: usize, used_counts: usize) -> Self {
        let pos = pos as isize;
        let ch_len = ch_len as isize;
        let used_counts = used_counts as isize;

        // 预测该 ch 压缩后的长度
        let cost = (pos / 52).max(1);

        // 固定代价
        // var , var 的的代价不进行计算

        // .foo => var a = "foo"; [a]
        // 使用一次的最小代价 `="",[]`
        let v1 = 1 + 2 + 1 + 2 + (cost * 2) - 1;
        // 后续使用的代价
        let v2 = (cost + 2 - 1) - ch_len;
        let v3 = v2 * (used_counts - 1);

        Self {
            first_cost: v1 as usize,
            more_cost: v2,
            original_cost: (ch_len * used_counts),
            compress_cost: v1 + v3,
        }
    }

    fn should_compress(&self) -> bool {
        self.compress_cost < 0
    }
}

#[cfg(test)]
mod tests {
    // use crate::count_cost;
    use super::*;

    #[test]
    fn f1() {
        let v = IdentCost::new(0, 3, 1);

        assert_eq!(v.first_cost, 7);
        assert_eq!(v.more_cost, -1);
        assert_eq!(v.original_cost, 3);
        assert_eq!(v.compress_cost, 7);
        assert!(!v.should_compress());

        let v = IdentCost::new(0, 3, 8);

        assert_eq!(v.first_cost, 7);
        assert_eq!(v.more_cost, -1);
        assert_eq!(v.original_cost, 24);
        assert_eq!(v.compress_cost, 0);
        assert!(!v.should_compress());

        let v = IdentCost::new(0, 3, 20);
        assert_eq!(v.first_cost, 7);
        assert_eq!(v.more_cost, -1);
        assert_eq!(v.original_cost, 60);
        assert_eq!(v.compress_cost, -12);
        assert!(v.should_compress());

        let v = IdentCost::new(0, 3, 100);
        assert_eq!(v.first_cost, 7);
        assert_eq!(v.more_cost, -1);
        assert_eq!(v.original_cost, 300);
        assert_eq!(v.compress_cost, -92);
        assert!(v.should_compress());
    }

    #[test]
    fn cannot_compress() {
        let map = HashMap::from_iter([
            ("aaa".to_string(), 1),
            ("bbb".to_string(), 1),
            ("ccc".to_string(), 1),
            ("ddd".to_string(), 1),
            ("eee".to_string(), 1),
        ]);

        let v = process_ident_map(map);

        assert_eq!(v, HashMap::new());
    }

    #[test]
    fn cannot_compress_long_but_used_once() {
        let map = HashMap::from_iter([(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            1,
        )]);

        let v = process_ident_map(map);

        assert_eq!(v, HashMap::new());
    }

    #[test]
    fn compress_long_ident_but_low_used() {
        let s = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string();
        let map = HashMap::from_iter([(s.clone(), 2)]);

        let v = process_ident_map(map);

        assert_eq!(v, HashMap::from_iter([(s, 2)]));
    }

    mod idents {
        use super::TokenAllocator;

        #[test]
        fn ident_alloc() {
            let mut token = TokenAllocator::new();

            let v = (0..200).map(|_| token.alloc()).collect::<Vec<_>>();

            println!("{:#?}", v);
        }
    }
}

#[derive(Debug, Default)]
struct ObjectMemberCollector {
    field: HashMap<String, usize>,
}

impl ObjectMemberCollector {
    fn new() -> Self {
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

impl Visit for ObjectMemberCollector {
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

#[derive(Debug)]
struct TokenAllocator {
    pos: usize,
    used_allocator: HashSet<String>,
}

impl TokenAllocator {
    fn new() -> Self {
        Self {
            pos: 0,
            used_allocator: HashSet::new(),
        }
    }

    fn ident(&self) -> String {
        let mut pos = self.pos / 52;
        let mut r = String::new();

        let mut push_ch = |ch: u8| {
            r.push(if ch < 26 {
                (b'a' + ch) as char
            } else {
                (b'A' + (ch - 26)) as char
            });
        };

        while pos > 0 {
            let ch = (pos - 1) % 52;

            push_ch(ch as u8);

            pos /= 52;
        }

        push_ch((self.pos % 52) as u8);

        r
    }

    fn alloc(&mut self) -> String {
        let s = self.ident();
        self.pos += 1;
        s
    }
}

#[derive(Debug)]
struct ObjectMemberReplacer {
    should_replace_idents: HashSet<String>,
    ident_map: HashMap<String, String>,
    allocator: TokenAllocator,
}

impl ObjectMemberReplacer {
    fn new(map: HashSet<String>) -> Self {
        Self {
            should_replace_idents: map,
            allocator: TokenAllocator::new(),
            ident_map: HashMap::new(),
        }
    }
}

impl ObjectMemberReplacer {
    fn contain(&self, ident: &str) -> bool {
        self.should_replace_idents.contains(ident)
    }

    fn create_ident_by_str(ident: &str) -> Ident {
        Ident {
            sym: ident.into(),
            span: Default::default(),
            optional: false,
            ctxt: Default::default(),
        }
    }

    fn create_ident(&mut self, ident: &str) -> Ident {
        // self.ident_map.entry(ident.to_string());

        if let Some(v) = self.ident_map.get(ident) {
            return Self::create_ident_by_str(v);
        }

        let s = self.allocator.alloc();

        let v = Self::create_ident_by_str(&s);

        self.ident_map.insert(ident.to_string(), s);

        v
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

impl VisitMut for ObjectMemberReplacer {
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
                // Prop::Computed(_) => {}
                _ => {}
            },
            PropOrSpread::Spread(_) => {}
        }
    }
}
