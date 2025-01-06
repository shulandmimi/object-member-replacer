#![deny(clippy::all)]
#![feature(box_patterns)]
#![feature(let_chains)]

mod collector;
mod replacer;
mod transformer;
mod util;

#[cfg(not(feature = "node"))]
pub use transformer::*;

#[cfg(feature = "node")]
#[macro_use]
extern crate napi_derive;

#[cfg(feature = "node")]
mod node {
    use super::*;

    #[napi(object)]
    pub struct TransformResult {
        pub content: String,
        pub map: Option<String>,
    }

    impl From<transformer::TransformResult> for TransformResult {
        fn from(result: transformer::TransformResult) -> Self {
            TransformResult {
                content: result.content,
                map: result.map,
            }
        }
    }

    #[napi]
    pub fn transform(
        content: String,
        options: serde_json::Value,
    ) -> anyhow::Result<TransformResult> {
        let options = serde_json::from_value(options).map_err(|e| anyhow::anyhow!(e))?;
        Ok(TransformResult::from(transformer::transform(
            content, options,
        )?))
    }
}

#[cfg(feature = "node")]
pub use node::*;
