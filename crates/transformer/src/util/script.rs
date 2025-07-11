use std::sync::Arc;

use anyhow::Result;
use swc_common::{
    input::SourceFileInput, source_map::SourceMapGenConfig, BytePos, FileName, LineCol, SourceFile,
    SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_codegen::{
    text_writer::{JsWriter, WriteJs},
    Config, Emitter,
};
use swc_ecma_parser::{lexer::Lexer, Parser, Syntax};

use crate::transformer::TransformContext;

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

pub fn parse(source_file: &SourceFile, syntax: Syntax) -> Result<Module> {
    let source_file_input = SourceFileInput::from(source_file);
    let lexer = Lexer::new(syntax, Default::default(), source_file_input, None);

    let mut parser = Parser::new_from(lexer);

    parser.parse_module().map_err(|err| {
        let msg = err.kind().msg();
        panic!("Parser Error: {}", msg);
    })
}

struct SourceMapConfig {}

impl SourceMapGenConfig for SourceMapConfig {
    fn file_name_to_source(&self, f: &FileName) -> String {
        f.to_string()
    }
    fn inline_sources_content(&self, _f: &FileName) -> bool {
        true
    }
}

pub fn try_build_output_sourcemap(
    source_map: Arc<SourceMap>,
    input_src: Arc<TransformContext>,
    src_map: Option<Vec<(BytePos, LineCol)>>,
) -> Result<Option<String>> {
    let Some(src) = src_map else {
        return Ok(None);
    };

    // SourceMap::from(value)
    let input_src = input_src
        .options
        .source_map
        .as_ref()
        .map(|s| sourcemap::SourceMap::from_slice(s.as_bytes()).ok())
        .flatten();

    let source_map =
        source_map.build_source_map_with_config(&src, input_src.as_ref(), SourceMapConfig {});

    let mut src_map = vec![];
    source_map.to_writer(&mut src_map)?;
    Ok(Some(String::from_utf8_lossy(&src_map).to_string()))
}
