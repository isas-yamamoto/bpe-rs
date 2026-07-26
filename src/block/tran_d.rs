//! TranD: per-band descendant significance transitions.

use crate::block::common::{band_origin, push_symbol_bit, ScanCtx};
use crate::error::{BpeError, BpeResult};
use crate::types::{BitPlaneBits, ENUM_TRAN_D};

pub(crate) fn scan_tran_d(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) -> BpeResult<()> {
    if block.str_plane_hit_history.tran_b == 1 {
        for k in 0..3usize {
            if ctx.band_excluded(&ctx.wt2, k) && ctx.band_excluded(&ctx.wt1, k) {
                continue;
            }
            if (block.str_plane_hit_history.tran_d & (1 << (2 - k))) != 0 {
                continue;
            }

            if !ctx.band_excluded(&ctx.wt2, k) {
                let (tx, ty) = band_origin(k, 2);
                if ctx.region_significant(block, tx, ty, 2) {
                    block.str_plane_hit_history.tran_d += 1 << (2 - k);
                    let sym = &mut block.symbols_block[si];
                    sym.type_ = ENUM_TRAN_D;
                    push_symbol_bit(sym, true);
                    continue;
                }
            }

            if ctx.band_excluded(&ctx.wt1, k) {
                continue;
            }
            let (tx, ty) = band_origin(k, 4);
            if ctx.region_significant(block, tx, ty, 4) {
                block.str_plane_hit_history.tran_d += 1 << (2 - k);
                let sym = &mut block.symbols_block[si];
                sym.type_ = ENUM_TRAN_D;
                push_symbol_bit(sym, true);
            } else if (block.str_plane_hit_history.tran_d & (1 << (2 - k))) == 0 {
                let sym = &mut block.symbols_block[si];
                sym.type_ = ENUM_TRAN_D;
                push_symbol_bit(sym, false);
            }
        }
    }

    if block.str_plane_hit_history.tran_d == 0 {
        return Err(BpeError::BlockScanCodingError);
    }
    Ok(())
}
