#![deny(clippy::all)]
#![feature(box_patterns)]

mod collector;
mod replacer;
mod transformer;

#[cfg(not(feature = "node"))]
pub use transformer::*;

#[cfg(feature = "node")]
#[macro_use]
extern crate napi_derive;

#[cfg(feature = "node")]
#[napi]
pub fn transform(content: String, options: serde_json::Value) -> anyhow::Result<String> {
    let options = serde_json::from_value(options).unwrap();
    transformer::transform(content, options)
}
