mod compress_ident;
mod token_allocator;
mod replacer;

pub use compress_ident::{filter_cannot_compress_ident, IdentCost};
pub use token_allocator::TokenAllocator;
// pub use replacer::IdentReplacer;