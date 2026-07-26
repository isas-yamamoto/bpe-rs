//! TranGi: grandchild significance transition per band.

use crate::block::common::{band_origin, push_symbol_bit, ScanCtx};
use crate::types::{BitPlaneBits, ENUM_TRAN_GI};

pub(crate) fn scan_tran_gi(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) {
    for k in 0..3usize {
        if ctx.band_excluded(&ctx.wt1, k) {
            continue;
        }
        if (block.str_plane_hit_history.tran_d & (1 << (2 - k))) != 0
            && (block.str_plane_hit_history.tran_gi & (1 << (2 - k))) == 0
        {
            block.symbols_block[si].type_ = ENUM_TRAN_GI;
            let (tx, ty) = band_origin(k, 4);
            if ctx.region_significant(block, tx, ty, 4) {
                push_symbol_bit(&mut block.symbols_block[si], true);
                block.str_plane_hit_history.tran_gi += 1 << (2 - k);
            } else {
                push_symbol_bit(&mut block.symbols_block[si], false);
            }
        }
    }
}
