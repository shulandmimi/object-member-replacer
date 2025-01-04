use std::sync::Arc;

use anyhow::Result;
use swc_common::{
    errors::HANDLER, BytePos, Globals, LineCol, Mark, SourceMap, SyntaxContext, GLOBALS,
};
use swc_common::{source_map::SourceMapGenConfig, FileName};
use swc_ecma_ast::Module;
use swc_ecma_transforms::{
    helpers::{Helpers, HELPERS},
    resolver,
};
use swc_ecma_visit::{VisitMut, VisitMutWith};
use swc_error_reporters::handler::try_with_handler;

struct ResetSyntaxContext;

impl VisitMut for ResetSyntaxContext {
    fn visit_mut_syntax_context(&mut self, syntax: &mut SyntaxContext) {
        syntax.remove_mark();
    }
}

pub fn resolve_module_mark(
    ast: &mut Module,
    is_typescript: bool,
    globals: &Globals,
) -> (Mark, Mark) {
    GLOBALS.set(globals, || {
        ast.visit_mut_with(&mut ResetSyntaxContext);

        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        ast.visit_mut_with(&mut resolver(
            unresolved_mark,
            top_level_mark,
            is_typescript,
        ));

        (unresolved_mark, top_level_mark)
    })
}

pub fn try_with<F>(cm: Arc<SourceMap>, globals: &Globals, op: F) -> Result<()>
where
    F: FnOnce(),
{
    GLOBALS.set(globals, || {
        try_with_handler(cm.clone(), Default::default(), |handler| {
            // swc_common::errors::HEL
            HELPERS.set(&Helpers::new(true), || HANDLER.set(handler, op));
            Ok(())
        })
    })
}

struct SourceMapConfig;

impl SourceMapGenConfig for SourceMapConfig {
    fn file_name_to_source(&self, f: &FileName) -> String {
        f.to_string()
    }
}

pub fn build_source_map(
    cm: Arc<SourceMap>,
    mappings: &[(BytePos, LineCol)],
) -> sourcemap::SourceMap {
    cm.build_source_map_with_config(mappings, None, SourceMapConfig)
}
