use std::{cell::OnceCell, path::Path, sync::Arc};

use itertools::Itertools;
use omm_core::filter_cannot_compress_ident;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use swc_common::{FileName, Globals};
use swc_ecma_ast::{
    BindingIdent, Decl, Expr, Lit, Module, ModuleItem, Pat, Stmt, VarDecl, VarDeclKind,
    VarDeclarator,
};
use swc_ecma_parser::{EsSyntax, Syntax, TsSyntax};
use swc_ecma_visit::{VisitMutWith, VisitWith};

use crate::{
    filter::{IdentFilterPlugin, IdentFilterPluginAdapter, IdentItem},
    optimize::gzip::GzipFilter,
    replacer::IdentReplacerConfig,
    util::{
        resolve_module_mark,
        script::{codegen, parse, try_build_output_sourcemap},
        try_with,
    },
};

use super::{collector::IdentCollector, replacer::IdentReplacer};

type Result<T> = anyhow::Result<T>;

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum ModuleType {
    #[default]
    Typescript,
    Javascript,
}

pub fn module_type_from_option(options: &TransformOption) -> ModuleType {
    match options.module_type {
        Some(ModuleType::Typescript) => ModuleType::Typescript,
        Some(ModuleType::Javascript) => ModuleType::Javascript,
        None => options
            .filename
            .as_ref()
            .map_or(ModuleType::Typescript, |filename| {
                if [".ts", ".tsx", ".mts", ".cts"]
                    .iter()
                    .any(|ext| filename.ends_with(ext))
                {
                    return ModuleType::Typescript;
                }

                ModuleType::Javascript
            }),
    }
}

pub fn syntax_from_option(module_type: &ModuleType) -> Syntax {
    match module_type {
        ModuleType::Typescript => Syntax::Es(EsSyntax::default()),
        ModuleType::Javascript => Syntax::Typescript(TsSyntax::default()),
    }
}

pub fn hosting_variable(module: &mut Module, replacer: IdentReplacer) {
    let mut decls: Vec<VarDeclarator> = vec![];
    for (val, key) in replacer
        .ident_map
        .into_iter()
        .sorted_by_key(|(_, ident)| ident.to_string())
    {
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

    if decls.is_empty() {
        return;
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
}

pub fn object_member_minify(
    module: &mut Module,
    context: &TransformContext,
    plugin: &IdentFilterPluginAdapter,
) {
    let (unresolved_mark, top_level_mark) = resolve_module_mark(
        module,
        matches!(context.module_type, ModuleType::Typescript),
        &context.globals,
    );

    // collection
    let mut collector = IdentCollector::new(unresolved_mark, top_level_mark).with_context(context);

    module.visit_with(&mut collector);

    let IdentCollector {
        mut field,
        used_ident,
        skip_lits,
        skip_ranges,
        ..
    } = collector;

    field.iter_mut().for_each(|(key, spans)| {
        spans.retain(|span| {
            !plugin.filter_ident(&&IdentItem {
                ident: key,
                range: (span.lo.0 as isize, span.hi.0 as isize),
            })
        });
    });

    let filterable_map = field
        .iter()
        .map(|(ident, set)| (ident.clone(), set.len()))
        .collect::<FxHashMap<_, _>>();

    // filter does not have to be replaced
    let map = filter_cannot_compress_ident(filterable_map);

    if map.is_empty() {
        return;
    }

    let keys = field.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        if !map.contains_key(&key) {
            field.remove(&key);
        }
    }

    drop(map);

    // replace ident
    let mut replacer = IdentReplacer::new(
        field.into_iter().map(|(k, spans)| (k, spans)).collect(),
        IdentReplacerConfig {
            skip_lits,
            skip_ranges,
        },
    )
    .with_context(context);

    replacer.extend_used_ident(used_ident);
    module.visit_mut_with(&mut replacer);

    // insert replaced ident
    hosting_variable(module, replacer);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", default = "Default::default")]
pub struct MemberMatchOption {
    ///
    /// The path of the word to ignore.
    ///
    ///
    ///
    /// try match
    /// ```unknown
    ///
    /// // path: "_require"
    /// _require.async("./foo")
    /// ^^^^^^^^
    ///
    /// // path: "foo.bar.foo1.bar1"
    /// foo.bar.foo1.bar1("./foo")
    /// ^^^ ^^^ ^^^^ ^^^^
    /// ```
    pub path: String,
    ///
    /// ignore the subpath of the word. eg. `async`
    ///
    /// ```unknown
    /// _require.async("./foo")
    ///          ^^^^^
    /// ```
    ///
    /// default: `true`
    pub subpath: bool,
    ///
    /// ignore the literal argument of the word. eg. `"./foo"`
    ///
    /// ```unknown
    /// _require.async("./foo")
    ///                 ^^^^^
    /// ```
    ///
    /// default: `false`
    pub skip_lit_arg: bool,
    ///
    /// ```unknown
    /// `require.async("namespace", "m1", foo("nest_arg"))`
    ///                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// ```
    ///
    /// default: `false`
    pub skip_arg: bool,
}

impl Default for MemberMatchOption {
    fn default() -> Self {
        Self {
            path: "".to_string(),
            subpath: true,
            skip_lit_arg: false,
            skip_arg: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StringLitOptions {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum IgnoreWord {
    #[serde(rename = "stringLit")]
    StringLit(StringLitOptions),
    #[serde(rename = "member")]
    MemberMatch(MemberMatchOption),
    #[serde(untagged)]
    Simple(String),
}

impl IgnoreWord {
    pub fn path(&self) -> Option<&str> {
        match self {
            IgnoreWord::MemberMatch(options) => Some(&options.path),
            IgnoreWord::Simple(v) => Some(v),
            IgnoreWord::StringLit(_) => None,
        }
    }

    pub fn subpath(&self) -> bool {
        match self {
            IgnoreWord::MemberMatch(options) => options.subpath,
            _ => MemberMatchOption::default().subpath,
        }
    }

    pub fn skip_lit_arg(&self) -> bool {
        match self {
            IgnoreWord::MemberMatch(options) => !self.skip_arg() && options.skip_lit_arg,
            _ => MemberMatchOption::default().skip_lit_arg,
        }
    }

    pub fn skip_arg(&self) -> bool {
        match self {
            IgnoreWord::MemberMatch(options) => options.skip_arg,
            _ => MemberMatchOption::default().skip_arg,
        }
    }
}

impl<T: AsRef<str>> From<T> for IgnoreWord {
    fn from(value: T) -> Self {
        IgnoreWord::Simple(value.as_ref().to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GzipOption {
    pub compress: Option<usize>,
    pub filter_level: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CompressionOption {
    #[serde(rename = "gzip")]
    Gzip(GzipOption),
}

impl Default for CompressionOption {
    fn default() -> Self {
        CompressionOption::Gzip(GzipOption::default())
    }
}

impl Default for GzipOption {
    fn default() -> Self {
        Self {
            compress: Default::default(),
            filter_level: Some(2.0),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct Optimize {
    pub compression: Option<CompressionOption>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransformOption {
    pub filename: Option<String>,
    pub source_map: Option<String>,
    #[serde(default)]
    pub enable_source_map: bool,
    pub module_type: Option<ModuleType>,

    #[serde(default)]
    pub preserve_keywords: Vec<String>,

    // TODO: support ignore object and object ident
    #[serde(default)]
    pub ignore_words: Vec<IgnoreWord>,

    #[serde(default)]
    pub optimize: Option<Optimize>,
}

impl TransformOption {
    fn filename(&self) -> String {
        self.filename.clone().unwrap_or("input.js".to_string())
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformResult {
    pub content: String,
    pub map: Option<String>,
}

#[allow(dead_code)]
pub struct TransformContext {
    pub module_type: ModuleType,
    pub options: TransformOption,
    pub globals: Arc<Globals>,
}

#[allow(clippy::declare_interior_mutable_const)]
const SWC_GLOBALS: OnceCell<Arc<Globals>> = OnceCell::new();

fn create_plugin_adapter(
    content: Arc<String>,
    context: &Arc<TransformContext>,
) -> IdentFilterPluginAdapter {
    let mut plugin_adapter = IdentFilterPluginAdapter::new(vec![]);

    if let Some(gzip_filter) = GzipFilter::new(content.clone(), context.clone()) {
        plugin_adapter = plugin_adapter.with_plugin(Box::new(gzip_filter) as _);
    }

    plugin_adapter
}

pub fn transform(content: String, options: TransformOption) -> Result<TransformResult> {
    let context = Arc::new(TransformContext {
        module_type: module_type_from_option(&options),
        options,
        #[allow(clippy::borrow_interior_mutable_const)]
        globals: SWC_GLOBALS.get_or_init(|| Arc::new(Globals::new())).clone(),
    });
    let filename = context.options.filename();

    let content = Arc::new(content);
    let syntax = syntax_from_option(&context.module_type);

    let filter_plugin_adapter = create_plugin_adapter(content.clone(), &context);

    let source_file_name = Arc::new(FileName::Real(Path::new(&filename).to_path_buf()));
    let source_map = Arc::new(swc_common::SourceMap::default());
    let source_file = source_map.new_source_file_from(source_file_name, content.clone());

    // parse
    let mut module = parse(&source_file, syntax)?;

    // optimize
    try_with(source_map.clone(), &context.globals.clone(), || {
        object_member_minify(&mut module, &context, &filter_plugin_adapter);
    })?;

    let mut src = if context.options.source_map.is_some() || context.options.enable_source_map {
        Some(vec![])
    } else {
        None
    };

    // codegen
    let code = codegen(&mut module, source_map.clone(), src.as_mut())?;

    let content = String::from_utf8_lossy(&code).to_string();
    let map = try_build_output_sourcemap(source_map, context, src)?;

    Ok(TransformResult { content, map })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() -> Result<()> {
        let input = r#"const obj = {}
obj.fooooooooooooooooooooooooooooooooooooooo = 1
obj["fooooooooooooooooooooooooooooooooooooooo"] = 1
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
console.log(obj.fooooooooooooooooooooooooooooooooooooooo)
"#;

        let result = transform(input.to_string(), Default::default())?;

        println!("{}", result.content);

        Ok(())
    }

    #[test]
    fn require_fields() -> Result<()> {
        fn create_result(options: TransformOption) -> Result<TransformResult> {
            transform(
                r#"
    require.async("./foo.js");
    require.async("./foo.js");
    require.async("./foo.js");
    require.async("./foo.js");
    require.async("./foo.js");
    require.async("./foo.js");
    require.async("./foo.js");
    "#
                .to_string(),
                options,
            )
        }

        fn assert_result(options: TransformOption, snapshot: &str) -> Result<()> {
            let result = create_result(options)?;

            assert_eq!(result.content.trim(), snapshot.trim());
            Ok(())
        }

        assert_result(
            TransformOption {
                ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                    path: "require".to_string(),
                    subpath: true,
                    skip_lit_arg: true,
                    ..Default::default()
                })],
                ..Default::default()
            },
            r#"var a = "async";
require[a]("./foo.js");
require[a]("./foo.js");
require[a]("./foo.js");
require[a]("./foo.js");
require[a]("./foo.js");
require[a]("./foo.js");
require[a]("./foo.js");"#,
        )?;

        assert_result(
            TransformOption {
                ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                    path: "require".to_string(),
                    subpath: false,
                    skip_lit_arg: false,
                    ..Default::default()
                })],
                ..Default::default()
            },
            r#"var a = "./foo.js";
require.async(a);
require.async(a);
require.async(a);
require.async(a);
require.async(a);
require.async(a);
require.async(a);"#,
        )?;

        Ok(())
    }

    #[test]
    fn require() -> Result<()> {
        let input = r#"
require("./foo.js");
require("./foo.js");
require("./foo.js");
require("./foo.js");
require("./foo.js");
require("./foo.js");
require("./foo.js");
        "#;

        let result = transform(
            input.to_string(),
            TransformOption {
                ignore_words: vec![IgnoreWord::MemberMatch(MemberMatchOption {
                    path: "require".to_string(),
                    subpath: false,
                    skip_lit_arg: true,
                    ..Default::default()
                })],
                ..Default::default()
            },
        )?;

        assert_eq!(result.content.trim(), input.trim());

        Ok(())
    }
}
