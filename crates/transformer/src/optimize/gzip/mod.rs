use std::{io::Read, sync::Arc};

use crate::{
    filter::IdentFilterPlugin,
    optimize::gzip::main::{
        inflate, Position, SegmentLeaf, SegmentNode, SegmentRange, SegmentTreeNode,
    },
    transformer::{CompressionOption, TransformContext},
};

mod git_bits;
mod main;
mod util;

pub struct GzipFilter {
    segment_tree: Arc<SegmentNode<f64>>,
    // context: Arc<TransformContext>,
}

impl GzipFilter {
    pub fn new(content: Arc<String>, context: Arc<TransformContext>) -> Option<Self> {
        let gzip_option = context.options.optimize.as_ref().and_then(|v| {
            #[allow(unreachable_patterns)]
            v.compression.as_ref().and_then(|opt| match opt {
                CompressionOption::Gzip(gzip) => Some(gzip),
                _ => None,
            })
        });

        if gzip_option.is_none() {
            return None;
        }

        let filter_level = gzip_option.map(|v| v.filter_level.unwrap_or(2.0));

        let mut v =
            flate2::read::GzEncoder::new(content.as_bytes(), flate2::Compression::default());

        let mut ret = vec![];

        v.read_to_end(&mut ret).unwrap();

        let ranges = inflate(ret);

        let mut segment_tree = SegmentNode::<f64>::Node(SegmentTreeNode::default());
        // .unwrap_or(2.0);
        for Position {
            start,
            end,
            bits,
            reference,
        } in ranges
        {
            if filter_level.is_some_and(|v| bits >= v) {
                continue;
            }

            segment_tree.insert(SegmentLeaf {
                start: start as isize,
                end: end as isize,
                data: Some(bits),
            });

            reference.into_iter().for_each(|(start, end)| {
                segment_tree.insert(SegmentLeaf {
                    start: start as isize,
                    end: end as isize,
                    data: None,
                });
            });
        }

        Some(Self {
            segment_tree: Arc::new(segment_tree),
        })
    }
}

impl IdentFilterPlugin for GzipFilter {
    fn filter_ident(&self, _ident: &crate::filter::IdentItem<'_>) -> bool {
        self.segment_tree.contain(&SegmentRange {
            start: _ident.range.0 as isize,
            end: _ident.range.1 as isize,
        })
    }
}
