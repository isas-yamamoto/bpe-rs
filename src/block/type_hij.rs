//! TypeHij: individual grandchildren of each significant 2x2 group.

use crate::block::common::{
    advance_if_used, append_grand_children_refine, grand_child_origin, push_symbol_bit,
    push_symbol_sign, ScanCtx,
};
use crate::types::{BitPlaneBits, ENUM_TYPE_HIJ};

pub(crate) fn scan_type_hij(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
    for i in 0..3usize {
        if ctx.band_excluded(&ctx.wt1, i) {
            continue;
        }
        for j in 0..4usize {
            if (block.str_plane_hit_history.tran_hi[i].tran_h & (1 << (3 - j))) == 0 {
                continue;
            }
            let (tx, ty) = grand_child_origin(i, j);

            let mut counter: u8 = 0;
            for k in 0..4usize {
                if (block.str_plane_hit_history.type_hij[i].type_hij[j].tran_h & (1 << (3 - k)))
                    == 0
                {
                    counter += 1;
                }
            }

            if counter == 0 {
                // Refinement bits: all four were hit before.
                for k in tx..tx + 2 {
                    for p in ty..ty + 2 {
                        if ctx.refine_included(&ctx.wt1, i) {
                            let bit = ctx.plane_bit(block.block_int[k][p]);
                            append_grand_children_refine(block, i, bit);
                        }
                    }
                }
                continue;
            }

            // Four grandchildren TypeHij will be scanned.
            advance_if_used(block, si);
            block.symbols_block[*si].type_ = ENUM_TYPE_HIJ;

            let mut t: u8 = 0;
            for k in tx..tx + 2 {
                for p in ty..ty + 2 {
                    let val = block.block_int[k][p];
                    if (block.str_plane_hit_history.type_hij[i].type_hij[j].tran_h & (1 << (3 - t)))
                        == 0
                    {
                        if ctx.is_significant(val) {
                            block.str_plane_hit_history.type_hij[i].type_hij[j].tran_h +=
                                1 << (3 - t);
                            let sym = &mut block.symbols_block[*si];
                            push_symbol_bit(sym, true);
                            push_symbol_sign(sym, val);
                        } else {
                            push_symbol_bit(&mut block.symbols_block[*si], false);
                        }
                    } else if ctx.refine_included(&ctx.wt1, i) {
                        let bit = ctx.plane_bit(val);
                        append_grand_children_refine(block, i, bit);
                    }
                    t += 1;
                }
            }
        }
    }
}
