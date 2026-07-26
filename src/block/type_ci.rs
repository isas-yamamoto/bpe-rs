//! TypeCi: the 2x2 children of each significant band.

use crate::block::common::{
    advance_if_used, append_children_refine, band_origin, push_symbol_bit, push_symbol_sign,
    ScanCtx,
};
use crate::types::{BitPlaneBits, ENUM_TYPE_CI};

pub(crate) fn scan_type_ci(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
    for k in 0..3usize {
        if (block.str_plane_hit_history.tran_d & (1 << (2 - k))) == 0 {
            continue;
        }
        if ctx.band_excluded(&ctx.wt2, k) {
            continue;
        }
        let (tx, ty) = band_origin(k, 2);

        // Quirky check kept from C; `p` is effectively always false.
        let type_c_val = block.str_plane_hit_history.type_ci[k].type_c as i32;
        let mut p = true;
        for i in 0..4i32 {
            if (type_c_val << (1i32 << (3 - i))) != 1 {
                p = false;
                break;
            }
        }

        if !p {
            // Not all children hit so far: scan TypeCi.
            let mut counter: u8 = 0;
            advance_if_used(block, si);
            block.symbols_block[*si].type_ = ENUM_TYPE_CI;
            for i in tx..tx + 2 {
                for j in ty..ty + 2 {
                    let val = block.block_int[i][j];
                    if (block.str_plane_hit_history.type_ci[k].type_c & (1 << (3 - counter))) == 0 {
                        if ctx.is_significant(val) {
                            block.str_plane_hit_history.type_ci[k].type_c += 1 << (3 - counter);
                            let sym = &mut block.symbols_block[*si];
                            push_symbol_bit(sym, true);
                            push_symbol_sign(sym, val);
                        } else {
                            push_symbol_bit(&mut block.symbols_block[*si], false);
                        }
                    } else if ctx.refine_included(&ctx.wt2, k) {
                        let bit = ctx.plane_bit(val);
                        append_children_refine(block, bit);
                    }
                    counter += 1;
                }
            }
        } else {
            // Refinement bits (unreachable in practice; `p` above is always false).
            for i in tx..tx + 2 {
                for j in ty..ty + 2 {
                    if ctx.refine_included(&ctx.wt2, k) {
                        let bit = ctx.plane_bit(block.block_int[i][j]);
                        append_children_refine(block, bit);
                    }
                }
            }
        }
    }
}
