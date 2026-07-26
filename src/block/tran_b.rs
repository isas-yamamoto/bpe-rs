//! TranB: any significant descendant in the whole block.

use crate::block::common::{advance_if_used, band_origin, ScanCtx};
use crate::types::{BitPlaneBits, SymbolDetails, ENUM_TRAN_B};

/// Write a 1-bit TranB symbol.
fn write_tran_b_symbol(sym: &mut SymbolDetails, bit: u8) {
    sym.sym_len = 1;
    sym.sym_val = bit;
    sym.type_ = ENUM_TRAN_B;
}

/// Returns false when the block has none (caller skips the remaining stages).
pub(crate) fn scan_tran_b(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) -> bool {
    if block.str_plane_hit_history.tran_b == 0 {
        'k_loop: for k in 0..3usize {
            if ctx.band_excluded(&ctx.wt2, k) && ctx.band_excluded(&ctx.wt1, k) {
                continue 'k_loop;
            }
            if !ctx.band_excluded(&ctx.wt2, k) {
                let (tx, ty) = band_origin(k, 2);
                if ctx.region_significant(block, tx, ty, 2) {
                    block.str_plane_hit_history.tran_b = 1;
                    write_tran_b_symbol(&mut block.symbols_block[*si], 1);
                    *si += 1;
                    // C: goto DS_Update
                    break 'k_loop;
                }
            }
            if ctx.band_excluded(&ctx.wt1, k) {
                continue 'k_loop;
            }
            let (tx, ty) = band_origin(k, 4);
            if ctx.region_significant(block, tx, ty, 4) {
                block.str_plane_hit_history.tran_b = 1;
                write_tran_b_symbol(&mut block.symbols_block[*si], 1);
                *si += 1;
                // C: goto DS_Update
                break 'k_loop;
            }
        }
    }

    if block.str_plane_hit_history.tran_b == 0 {
        write_tran_b_symbol(&mut block.symbols_block[*si], 0);
        return false;
    }
    advance_if_used(block, si);
    true
}
