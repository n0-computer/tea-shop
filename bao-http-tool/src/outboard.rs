use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, RwLock},
};

use bao_tree::{blake3::Hash, iter::BaoChunk, BaoTree, ByteRanges, ChunkRangesRef, TreeNode};
use range_collections::{range_set::RangeSetRange, RangeSetRef};

struct SparseOutboardCache<T> {
    cache: Arc<RwLock<BTreeMap<TreeNode, (Hash, Hash)>>>,
    inner: T,
}

impl<T: bao_tree::io::fsm::Outboard> bao_tree::io::fsm::Outboard for SparseOutboardCache<T> {
    fn root(&self) -> bao_tree::blake3::Hash {
        self.inner.root()
    }

    fn tree(&self) -> bao_tree::BaoTree {
        self.inner.tree()
    }

    async fn load(
        &mut self,
        node: bao_tree::TreeNode,
    ) -> io::Result<Option<(bao_tree::blake3::Hash, bao_tree::blake3::Hash)>> {
        if let Some(entry) = self.cache.read().unwrap().get(&node) {
            return Ok(Some(entry.clone()));
        }
        let entry = self.inner.load(node).await?;
        if let Some(entry) = entry {
            self.cache.write().unwrap().insert(node, entry.clone());
        }
        Ok(entry)
    }
}

/// Get the set of node IDs needed to encode the given chunk ranges.
pub async fn node_ids(
    tree: BaoTree,
    ranges: &ChunkRangesRef,
) -> impl Iterator<Item = TreeNode> + '_ {
    // todo!
    // let ranges = bao_tree::rec::truncate_ranges(ranges, outboard.tree().size())
    tree.ranges_pre_order_chunks_iter_ref(ranges, tree.block_size().chunk_log())
        .filter_map(|chunk| match chunk {
            BaoChunk::Parent { node, .. } => Some(node),
            _ => None,
        })
}

/// Compute the byte ranges of a pre order offset without size prefix to encode the given nodes.
pub fn pre_order_ranges(tree: BaoTree, nodes: impl IntoIterator<Item = TreeNode>) -> ByteRanges {
    let mut res = ByteRanges::empty();
    for node in nodes {
        if let Some(offset) = tree.post_order_offset(node) {
            let offset = offset.value() * 64;
            res |= ByteRanges::from(offset..offset + 64);
        }
    }
    res
}

/// Rounds up ranges to the nearest chunk boundaries, where a chunk is 2^block_shift bytes.
pub fn simplify(ranges: &RangeSetRef<u64>, block_shift: u8) -> ByteRanges {
    let mut res = ByteRanges::empty();
    for item in ranges.iter() {
        match item {
            RangeSetRange::Range(range) => {
                let start = (range.start >> block_shift) << block_shift;
                let end = ((range.end + ((1 << block_shift) - 1)) >> block_shift) << block_shift;
                res |= ByteRanges::from(start..end);
            }
            RangeSetRange::RangeFrom(range) => {
                let start = (range.start >> block_shift) << block_shift;
                res |= ByteRanges::from(start..);
            }
        }
    }
    res
}

/// Prefetch all nodes in the tree that are needed to encode the given chunk ranges.
pub async fn prefetch(
    mut outboard: impl bao_tree::io::fsm::Outboard,
    ranges: &ChunkRangesRef,
) -> io::Result<()> {
    for node in node_ids(outboard.tree(), ranges).await {
        outboard.load(node).await?;
    }
    Ok(())
}
