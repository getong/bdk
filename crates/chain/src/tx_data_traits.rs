use crate::collections::{BTreeMap, BTreeSet};
use crate::{BlockId, ConfirmationBlockTime};
use alloc::vec::Vec;

/// Trait that "anchors" blockchain data to a specific block of height and hash.
///
/// If transaction A is anchored in block B, and block B is in the best chain, we can
/// assume that transaction A is also confirmed in the best chain. This does not necessarily mean
/// that transaction A is confirmed in block B. It could also mean transaction A is confirmed in a
/// parent block of B.
///
/// Every [`Anchor`] implementation must contain a [`BlockId`] parameter, and must implement
/// [`Ord`]. When implementing [`Ord`], the anchors' [`BlockId`]s should take precedence
/// over other elements inside the [`Anchor`]s for comparison purposes, i.e., you should first
/// compare the anchors' [`BlockId`]s and then care about the rest.
///
/// The example shows different types of anchors:
/// ```
/// # use bdk_chain::local_chain::LocalChain;
/// # use bdk_chain::tx_graph::TxGraph;
/// # use bdk_chain::BlockId;
/// # use bdk_chain::ConfirmationBlockTime;
/// # use bdk_chain::example_utils::*;
/// # use bitcoin::hashes::Hash;
/// // Initialize the local chain with two blocks.
/// let chain = LocalChain::from_blocks(
///     [
///         (1, Hash::hash("first".as_bytes())),
///         (2, Hash::hash("second".as_bytes())),
///     ]
///     .into_iter()
///     .collect(),
/// );
///
/// // Transaction to be inserted into `TxGraph`s with different anchor types.
/// let tx = tx_from_hex(RAW_TX_1);
///
/// // Insert `tx` into a `TxGraph` that uses `BlockId` as the anchor type.
/// // When a transaction is anchored with `BlockId`, the anchor block and the confirmation block of
/// // the transaction is the same block.
/// let mut graph_a = TxGraph::<BlockId>::default();
/// let _ = graph_a.insert_tx(tx.clone());
/// graph_a.insert_anchor(
///     tx.compute_txid(),
///     BlockId {
///         height: 1,
///         hash: Hash::hash("first".as_bytes()),
///     },
/// );
///
/// // Insert `tx` into a `TxGraph` that uses `ConfirmationBlockTime` as the anchor type.
/// // This anchor records the anchor block and the confirmation time of the transaction. When a
/// // transaction is anchored with `ConfirmationBlockTime`, the anchor block and confirmation block
/// // of the transaction is the same block.
/// let mut graph_c = TxGraph::<ConfirmationBlockTime>::default();
/// let _ = graph_c.insert_tx(tx.clone());
/// graph_c.insert_anchor(
///     tx.compute_txid(),
///     ConfirmationBlockTime {
///         block_id: BlockId {
///             height: 2,
///             hash: Hash::hash("third".as_bytes()),
///         },
///         confirmation_time: 123,
///     },
/// );
/// ```
pub trait Anchor: core::fmt::Debug + Clone + Eq + PartialOrd + Ord + core::hash::Hash {
    /// Returns the [`BlockId`] that the associated blockchain data is "anchored" in.
    fn anchor_block(&self) -> BlockId;

    /// Get the upper bound of the chain data's confirmation height.
    ///
    /// The default definition gives a pessimistic answer. This can be overridden by the `Anchor`
    /// implementation for a more accurate value.
    fn confirmation_height_upper_bound(&self) -> u32 {
        self.anchor_block().height
    }
}

impl<'a, A: Anchor> Anchor for &'a A {
    fn anchor_block(&self) -> BlockId {
        <A as Anchor>::anchor_block(self)
    }
}

impl Anchor for BlockId {
    fn anchor_block(&self) -> Self {
        *self
    }
}

impl Anchor for ConfirmationBlockTime {
    fn anchor_block(&self) -> BlockId {
        self.block_id
    }

    fn confirmation_height_upper_bound(&self) -> u32 {
        self.block_id.height
    }
}

/// An [`Anchor`] that can be constructed from a given block, block height and transaction position
/// within the block.
pub trait AnchorFromBlockPosition: Anchor {
    /// Construct the anchor from a given `block`, block height and `tx_pos` within the block.
    fn from_block_position(block: &bitcoin::Block, block_id: BlockId, tx_pos: usize) -> Self;
}

impl AnchorFromBlockPosition for BlockId {
    fn from_block_position(_block: &bitcoin::Block, block_id: BlockId, _tx_pos: usize) -> Self {
        block_id
    }
}

impl AnchorFromBlockPosition for ConfirmationBlockTime {
    fn from_block_position(block: &bitcoin::Block, block_id: BlockId, _tx_pos: usize) -> Self {
        Self {
            block_id,
            confirmation_time: block.header.time as _,
        }
    }
}

/// Trait that makes an object mergeable.
pub trait Merge: Default {
    /// Merge another object of the same type onto `self`.
    fn merge(&mut self, other: Self);

    /// Returns whether the structure is considered empty.
    fn is_empty(&self) -> bool;

    /// Take the value, replacing it with the default value.
    fn take(&mut self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(core::mem::take(self))
        }
    }
}

impl<K: Ord, V> Merge for BTreeMap<K, V> {
    fn merge(&mut self, other: Self) {
        // We use `extend` instead of `BTreeMap::append` due to performance issues with `append`.
        // Refer to https://github.com/rust-lang/rust/issues/34666#issuecomment-675658420
        BTreeMap::extend(self, other)
    }

    fn is_empty(&self) -> bool {
        BTreeMap::is_empty(self)
    }
}

impl<T: Ord> Merge for BTreeSet<T> {
    fn merge(&mut self, other: Self) {
        // We use `extend` instead of `BTreeMap::append` due to performance issues with `append`.
        // Refer to https://github.com/rust-lang/rust/issues/34666#issuecomment-675658420
        BTreeSet::extend(self, other)
    }

    fn is_empty(&self) -> bool {
        BTreeSet::is_empty(self)
    }
}

impl<T> Merge for Vec<T> {
    fn merge(&mut self, mut other: Self) {
        Vec::append(self, &mut other)
    }

    fn is_empty(&self) -> bool {
        Vec::is_empty(self)
    }
}

macro_rules! impl_merge_for_tuple {
    ($($a:ident $b:tt)*) => {
        impl<$($a),*> Merge for ($($a,)*) where $($a: Merge),* {

            fn merge(&mut self, _other: Self) {
                $(Merge::merge(&mut self.$b, _other.$b) );*
            }

            fn is_empty(&self) -> bool {
                $(Merge::is_empty(&self.$b) && )* true
            }
        }
    }
}

impl_merge_for_tuple!();
impl_merge_for_tuple!(T0 0);
impl_merge_for_tuple!(T0 0 T1 1);
impl_merge_for_tuple!(T0 0 T1 1 T2 2);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5 T6 6);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5 T6 6 T7 7);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5 T6 6 T7 7 T8 8);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5 T6 6 T7 7 T8 8 T9 9);
impl_merge_for_tuple!(T0 0 T1 1 T2 2 T3 3 T4 4 T5 5 T6 6 T7 7 T8 8 T9 9 T10 10);
