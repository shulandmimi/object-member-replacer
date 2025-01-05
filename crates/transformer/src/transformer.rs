use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
    sync::Arc,
};

use enhanced_magic_string::collapse_sourcemap::CollapseSourcemapOptions;
use itertools::Itertools;
use omm_core::filter_cannot_compress_ident;
use serde::{Deserialize, Serialize};
use swc_common::{BytePos, FileName, Globals, LineCol, SourceMap};
use swc_ecma_ast::{
    BindingIdent, Decl, Expr, Lit, Module, ModuleItem, Pat, Stmt, VarDecl, VarDeclKind,
    VarDeclarator,
};
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Config, Emitter,
};
use swc_ecma_parser::{EsSyntax, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{VisitMutWith, VisitWith};

use crate::util::{build_source_map, resolve_module_mark, try_with};

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

pub fn object_member_minify(module: &mut Module, context: &TransformContext) {
    let (unresolved_mark, top_level_mark) = resolve_module_mark(
        module,
        matches!(context.module_type, ModuleType::Typescript),
        &context.globals,
    );

    // collection
    let mut collector = IdentCollector::new(unresolved_mark, top_level_mark);

    module.visit_with(&mut collector);

    let IdentCollector {
        field,
        unresolved_ident,
        ..
    } = collector;

    // filter does not have to be replaced
    let map = filter_cannot_compress_ident(field);

    if map.is_empty() {
        return;
    }

    // replace ident
    let mut replacer = IdentReplacer::new(map.into_keys().collect());

    replacer.extend_used_ident(unresolved_ident);
    module.visit_mut_with(&mut replacer);

    // insert replaced ident
    hosting_variable(module, replacer);
}

pub fn parse(content: Arc<String>, syntax: Syntax) -> Result<Module> {
    let content = StringInput::new(&content, Default::default(), Default::default());

    let mut parser = Parser::new(syntax, content, None);

    parser.parse_module().map_err(|err| {
        let msg = err.kind().msg();
        panic!("Parser Error: {}", msg);
    })
}

pub fn codegen(
    module: &mut Module,
    cm: Arc<SourceMap>,
    src_map: Option<&mut Vec<(BytePos, LineCol)>>,
) -> Result<Vec<u8>> {
    let config = Config::default().with_omit_last_semi(true);
    let mut buf = vec![];
    // let src = Vec::new();
    let writer = Box::new(JsWriter::new(cm.clone(), "\n", &mut buf, src_map)) as Box<dyn WriteJs>;

    let mut emitter = Emitter {
        cfg: config,
        cm,
        comments: None,
        wr: writer,
    };

    emitter.emit_module(module)?;

    drop(emitter);

    Ok(buf)
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformOption {
    filename: Option<String>,
    source_map: Option<String>,
    #[serde(default)]
    enable_source_map: bool,
    module_type: Option<ModuleType>,
}

impl TransformOption {
    fn filename(&self) -> String {
        self.filename.clone().unwrap_or("input".to_string())
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
    module_type: ModuleType,
    options: TransformOption,
    globals: Arc<Globals>,
}

#[allow(clippy::declare_interior_mutable_const)]
const SWC_GLOBALS: OnceCell<Arc<Globals>> = OnceCell::new();

fn create_source_map(path: PathBuf, content: Arc<String>) -> SourceMap {
    let source_map = SourceMap::default();
    source_map.new_source_file_from(FileName::Real(path).into(), content);
    source_map
}

fn try_build_output_sourcemap(
    source_map: Arc<SourceMap>,
    input_src: Option<String>,
    src_map: Option<Vec<(BytePos, LineCol)>>,
) -> Result<Option<String>> {
    let Some(src) = src_map else {
        return Ok(None);
    };

    // after transform sourcemap
    let source_map = build_source_map(source_map.clone(), &src);
    let mut buf = vec![];
    source_map.to_writer(&mut buf)?;
    let source_map = String::from_utf8_lossy(&buf).to_string();

    // collapse input sourcemap and transform sourcemap
    let mut sourcemap_chains = vec![];
    let append_source_map = |s: String| sourcemap::SourceMap::from_slice(s.as_bytes());
    if let Some(input_src) = input_src {
        sourcemap_chains.push(append_source_map(input_src)?);
    }
    sourcemap_chains.push(append_source_map(source_map)?);
    let collapse_sourcemap = enhanced_magic_string::collapse_sourcemap::collapse_sourcemap_chain(
        sourcemap_chains,
        CollapseSourcemapOptions::default(),
    );

    let mut src_map = vec![];
    collapse_sourcemap.to_writer(&mut src_map)?;
    Ok(Some(String::from_utf8_lossy(&src_map).to_string()))
}

pub fn transform(content: String, options: TransformOption) -> Result<TransformResult> {
    let context = TransformContext {
        module_type: module_type_from_option(&options),
        options,
        #[allow(clippy::borrow_interior_mutable_const)]
        globals: SWC_GLOBALS.get_or_init(|| Arc::new(Globals::new())).clone(),
    };
    let filename = context.options.filename();

    let content = Arc::new(content);
    let syntax = syntax_from_option(&context.module_type);

    // parse
    let mut module = parse(content.clone(), syntax)?;

    let source_map = Arc::new(create_source_map(
        Path::new(&filename).to_path_buf(),
        content,
    ));

    // optimize
    try_with(source_map.clone(), &context.globals.clone(), || {
        object_member_minify(&mut module, &context);
    })?;

    let mut src = if context.options.source_map.is_some() || context.options.enable_source_map {
        Some(vec![])
    } else {
        None
    };

    // codegen
    let code = codegen(&mut module, source_map.clone(), src.as_mut())?;

    let content = String::from_utf8_lossy(&code).to_string();
    let map = try_build_output_sourcemap(source_map, context.options.source_map, src)?;

    Ok(TransformResult { content, map })
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() -> Result<()> {
        let input = r#"
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
    "#;

        let result = transform(input.to_string(), Default::default())?;

        println!("{}", result.content);

        Ok(())
    }
}
