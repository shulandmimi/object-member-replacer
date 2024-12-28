use std::rc::Rc;

use itertools::Itertools;
use omm_core::filter_cannot_compress_ident;
use serde::{Deserialize, Serialize};
use swc_common::SourceMap;
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

use super::{collector::IdentCollector, replacer::IdentReplacer};

type Result<T> = anyhow::Result<T>;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ModuleType {
    #[default]
    Typescript,
    Javascript,
}

pub fn syntax_from_option(options: &TransformOption) -> Syntax {
    let module_type = match options.module_type {
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
    };
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

pub fn object_member_minify(module: &mut Module) {
    // collection
    let mut collector = IdentCollector::new();
    module.visit_with(&mut collector);

    let IdentCollector { field } = collector;

    // filter does not have to be replaced
    let map = filter_cannot_compress_ident(field);

    // replace ident
    let mut replacer = IdentReplacer::new(map.into_keys().collect());
    module.visit_mut_with(&mut replacer);

    // insert replaced ident
    hosting_variable(module, replacer);
}

pub fn parse(content: Rc<String>, options: &TransformOption) -> Result<Module> {
    // parser
    let syntax = syntax_from_option(options);

    let content = StringInput::new(&content, Default::default(), Default::default());

    let mut parser = Parser::new(syntax, content, None);

    parser.parse_module().map_err(|err| {
        let msg = err.kind().msg();
        panic!("Parser Error: {}", msg);
    })
}

pub fn codegen(module: &mut Module) -> Result<Vec<u8>> {
    let config = Config::default().with_omit_last_semi(true);
    let cm = Rc::new(SourceMap::default());
    let mut buf = vec![];
    let writer = Box::new(JsWriter::new(cm.clone(), "\n", &mut buf, None)) as Box<dyn WriteJs>;

    let mut emitter = Emitter {
        cfg: config,
        cm: cm.clone(),
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
    module_type: Option<ModuleType>,
}

pub fn transform(content: String, options: TransformOption) -> Result<String> {
    let content = Rc::new(content);
    let mut module = parse(content, &options)?;

    // optimize
    object_member_minify(&mut module);

    // codegen
    let buf = codegen(&mut module)?;

    let result = String::from_utf8_lossy(&buf);

    Ok(result.to_string())
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

        println!("{}", result);

        Ok(())
    }
}
