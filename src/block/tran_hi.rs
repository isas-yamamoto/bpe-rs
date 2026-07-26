//! TranHi: 2x2 grandchild group transitions per band.

use crate::block::common::{advance_if_used, grand_child_origin, push_symbol_bit, ScanCtx};
use crate::types::{BitPlaneBits, ENUM_TRAN_HI};

pub(crate) fn scan_tran_hi(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
    for i in 0..3usize {
        if ctx.band_excluded(&ctx.wt1, i) {
            continue;
        }
        advance_if_used(block, si);
        for j in 0..4usize {
            if (block.str_plane_hit_history.tran_gi & (1 << (2 - i))) != 0
                && (block.str_plane_hit_history.tran_hi[i].tran_h & (1 << (3 - j))) == 0
            {
                block.symbols_block[*si].type_ = ENUM_TRAN_HI;
                let (tx, ty) = grand_child_origin(i, j);
                if ctx.region_significant(block, tx, ty, 2) {
                    push_symbol_bit(&mut block.symbols_block[*si], true);
                    block.str_plane_hit_history.tran_hi[i].tran_h += 1 << (3 - j);
                } else {
                    push_symbol_bit(&mut block.symbols_block[*si], false);
                }
            }
        }
    }
}
