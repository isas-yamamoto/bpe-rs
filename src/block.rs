//! Block bit-plane scanning - original/source/BPEBlockCoding.c

use crate::error::{BpeError, BpeResult};
use crate::types::{
    amplitude, sign_of, BitPlaneBits, CodingPara, SymbolDetails, ENUM_TRAN_B, ENUM_TRAN_D,
    ENUM_TRAN_GI, ENUM_TRAN_HI, ENUM_TYPE_CI, ENUM_TYPE_HIJ, ENUM_TYPE_P, INTEGER_WAVELET,
};

/// Per-plane scan context: current bit plane and custom subband weights.
struct ScanCtx {
    bit_plane: u8,
    bit_set_plane: u32,
    integer_wavelet: bool,
    /// Custom weights per DWT level, indexed by band (0 = HL, 1 = LH, 2 = HH).
    wt1: [u8; 3],
    wt2: [u8; 3],
    wt3: [u8; 3],
}

impl ScanCtx {
    fn new(coding: &CodingPara) -> Self {
        let p4 = &coding.header.part4;
        Self {
            bit_plane: coding.bit_plane,
            bit_set_plane: 1u32 << (coding.bit_plane - 1),
            integer_wavelet: p4.dwt_type == INTEGER_WAVELET,
            wt1: [p4.custom_wt_hl1, p4.custom_wt_lh1, p4.custom_wt_hh1],
            wt2: [p4.custom_wt_hl2, p4.custom_wt_lh2, p4.custom_wt_hh2],
            wt3: [p4.custom_wt_hl3, p4.custom_wt_lh3, p4.custom_wt_hh3],
        }
    }

    /// Integer wavelet only: band is still absorbed by the custom weight at this plane.
    fn band_excluded(&self, wt: &[u8; 3], band: usize) -> bool {
        self.integer_wavelet && wt[band] >= self.bit_plane
    }

    /// Whether a refinement bit of this band is emitted at this plane.
    fn refine_included(&self, wt: &[u8; 3], band: usize) -> bool {
        !self.integer_wavelet || wt[band] < self.bit_plane
    }

    /// Coefficient has its bit set at the current plane.
    fn is_significant(&self, val: i32) -> bool {
        (self.bit_set_plane & (amplitude(val) as u32)) > 0
    }

    /// Bit of `val` at the current plane (0 or 1).
    fn plane_bit(&self, val: i32) -> u16 {
        if ((amplitude(val) as u32) & self.bit_set_plane) > 0 {
            1
        } else {
            0
        }
    }

    /// True if any coefficient in the `size` x `size` region at (tx, ty) is significant.
    fn region_significant(&self, block: &BitPlaneBits, tx: usize, ty: usize, size: usize) -> bool {
        for i in tx..tx + size {
            for j in ty..ty + size {
                if self.is_significant(block.block_int[i][j]) {
                    return true;
                }
            }
        }
        false
    }
}

/// Top-left corner of a band region scaled by `scale` (0 = HL, 1 = LH, 2 = HH).
fn band_origin(band: usize, scale: usize) -> (usize, usize) {
    let x = if band >= 1 { 1 } else { 0 };
    let y = if band != 1 { 1 } else { 0 };
    (x * scale, y * scale)
}

/// Top-left corner of grandchild group `group` (0..4) inside `band`.
fn grand_child_origin(band: usize, group: usize) -> (usize, usize) {
    let (bx, by) = band_origin(band, 4);
    let gx = if group >= 2 { 2 } else { 0 };
    (bx + gx, by + (group % 2) * 2)
}

/// Append one bit to the symbol value.
fn push_symbol_bit(sym: &mut SymbolDetails, one: bool) {
    sym.sym_len += 1;
    sym.sym_val <<= 1;
    if one {
        sym.sym_val += 1;
    }
}

/// Append the sign of `val` to the symbol sign field.
fn push_symbol_sign(sym: &mut SymbolDetails, val: i32) {
    sym.sign <<= 1;
    sym.sign += sign_of(val);
}

/// Advance to the next symbol slot if the current one holds any bits.
fn advance_if_used(block: &BitPlaneBits, si: &mut usize) {
    if block.symbols_block[*si].sym_len != 0 {
        *si += 1;
    }
}

fn append_parent_refine(block: &mut BitPlaneBits, bit: u8) {
    let rp = &mut block.refine_bits.refine_parent;
    rp.parent_ref_symbol = (rp.parent_ref_symbol << 1) + bit;
    rp.parent_symbol_length += 1;
}

fn append_children_refine(block: &mut BitPlaneBits, bit: u16) {
    let rc = &mut block.refine_bits.refine_children;
    rc.children_ref_symbol = (rc.children_ref_symbol << 1) + bit;
    rc.children_symbol_length += 1;
}

fn append_grand_children_refine(block: &mut BitPlaneBits, band: usize, bit: u16) {
    let rg = &mut block.refine_bits.refine_grand_children[band];
    rg.grand_children_ref_symbol = (rg.grand_children_ref_symbol << 1) + bit;
    rg.grand_children_symbol_length += 1;
}

/// TypeP: the three parent coefficients.
fn scan_type_p(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) {
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

/// Write a 1-bit TranB symbol.
fn write_tran_b_symbol(sym: &mut SymbolDetails, bit: u8) {
    sym.sym_len = 1;
    sym.sym_val = bit;
    sym.type_ = ENUM_TRAN_B;
}

/// TranB: any significant descendant in the whole block.
/// Returns false when the block has none (caller skips the remaining stages).
fn scan_tran_b(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) -> bool {
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

/// TranD: per-band descendant significance transitions.
fn scan_tran_d(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) -> BpeResult<()> {
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

/// TypeCi: the 2x2 children of each significant band.
fn scan_type_ci(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
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

/// TranGi: grandchild significance transition per band.
fn scan_tran_gi(ctx: &ScanCtx, block: &mut BitPlaneBits, si: usize) {
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

/// TranHi: 2x2 grandchild group transitions per band.
fn scan_tran_hi(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
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

/// TypeHij: individual grandchildren of each significant 2x2 group.
fn scan_type_hij(ctx: &ScanCtx, block: &mut BitPlaneBits, si: &mut usize) {
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

pub fn block_scan_encode(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let ctx = ScanCtx::new(coding);
    let s = coding.header.part3.s_20bits as usize;

    for block_seq in 0..s {
        let block = &mut block_info[block_seq];
        if block.bit_max_ac < ctx.bit_plane as u16 {
            continue;
        }
        coding.block_index = block_seq as u32;

        let mut si: usize = 0;
        scan_type_p(&ctx, block, si);
        advance_if_used(block, &mut si);

        // No significant descendants at this plane: a zero TranB symbol ends the block.
        if !scan_tran_b(&ctx, block, &mut si) {
            continue;
        }

        scan_tran_d(&ctx, block, si)?;
        scan_type_ci(&ctx, block, &mut si);
        advance_if_used(block, &mut si);

        scan_tran_gi(&ctx, block, si);
        advance_if_used(block, &mut si);

        scan_tran_hi(&ctx, block, &mut si);
        scan_type_hij(&ctx, block, &mut si);
    }

    Ok(())
}
