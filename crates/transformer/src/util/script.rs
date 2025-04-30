use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use enhanced_magic_string::collapse_sourcemap::CollapseSourcemapOptions;
use swc_common::{BytePos, FileName, LineCol, SourceMap};
use swc_ecma_ast::Module;
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Config, Emitter,
};
use swc_ecma_parser::{Parser, StringInput, Syntax};

use crate::util::build_source_map;

pub fn codegen(
    module: &mut Module,
    cm: Arc<SourceMap>,
    src_map: Option<&mut Vec<(BytePos, LineCol)>>,
) -> Result<Vec<u8>> {
    let config = Config::default().with_omit_last_semi(true);
    let mut buf = vec![];
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

pub fn parse(content: Arc<String>, syntax: Syntax) -> Result<Module> {
    let content = StringInput::new(&content, Default::default(), Default::default());

    let mut parser = Parser::new(syntax, content, None);

    parser.parse_module().map_err(|err| {
        let msg = err.kind().msg();
        panic!("Parser Error: {}", msg);
    })
}

pub fn create_source_map(path: PathBuf, content: Arc<String>) -> SourceMap {
    let source_map = SourceMap::default();
    source_map.new_source_file_from(FileName::Real(path).into(), content);
    source_map
}

pub fn try_build_output_sourcemap(
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
