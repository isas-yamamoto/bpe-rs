//! TypeP: the three parent coefficients.

use crate::block::common::{
    append_parent_refine, band_origin, push_symbol_bit, push_symbol_sign, ScanCtx,
};
use crate::types::{amplitude, BitPlaneBits, ENUM_TYPE_P};

pub(crate) fn scan_type_p(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) {
    for i in 0..3usize {
        if ctx.band_excluded(&ctx.wt3, i) {
            continue;
        }
        let (x, y) = band_origin(i, 1);
        let val = block.block_int[x][y];

        if (block.str_plane_hit_history.type_p & (1 << (2 - i))) == 0 {
            block.symbols_block[si].type_ = ENUM_TYPE_P;
            let amp = amplitude(val);
            // Parent significance: this plane holds the most significant bit.
            let significant = amp >= (1i32 << (ctx.bit_plane - 1)) && amp < (1i32 << ctx.bit_plane);
            if significant {
                block.str_plane_hit_history.type_p += 1 << (2 - i);
            }
            let sym = &mut block.symbols_block[si];
            push_symbol_bit(sym, significant);
            if significant {
                push_symbol_sign(sym, val);
            }
        } else if ctx.refine_included(&ctx.wt3, i) {
            let bit = ctx.plane_bit(val) as u8;
            append_parent_refine(block, bit);
        }
    }
}
