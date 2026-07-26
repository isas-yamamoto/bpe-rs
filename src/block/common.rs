//! Shared scan context and symbol/refine helpers for block bit-plane scanning.

use crate::types::{amplitude, sign_of, BitPlaneBits, CodingPara, SymbolDetails, INTEGER_WAVELET};

/// Per-plane scan context: current bit plane and custom subband weights.
pub(crate) struct ScanCtx {
    pub bit_plane: u8,
    pub bit_set_plane: u32,
    pub integer_wavelet: bool,
    /// Custom weights per DWT level, indexed by band (0 = HL, 1 = LH, 2 = HH).
    pub wt1: [u8; 3],
    pub wt2: [u8; 3],
    pub wt3: [u8; 3],
}

impl ScanCtx {
    pub fn new(coding: &CodingPara) -> Self {
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
    pub fn band_excluded(&self, wt: &[u8; 3], band: usize) -> bool {
        self.integer_wavelet && wt[band] >= self.bit_plane
    }

    /// Whether a refinement bit of this band is emitted at this plane.
    pub fn refine_included(&self, wt: &[u8; 3], band: usize) -> bool {
        !self.integer_wavelet || wt[band] < self.bit_plane
    }

    /// Coefficient has its bit set at the current plane.
    pub fn is_significant(&self, val: i32) -> bool {
        (self.bit_set_plane & (amplitude(val) as u32)) > 0
    }

    /// Bit of `val` at the current plane (0 or 1).
    pub fn plane_bit(&self, val: i32) -> u16 {
        if ((amplitude(val) as u32) & self.bit_set_plane) > 0 {
            1
        } else {
            0
        }
    }

    /// True if any coefficient in the `size` x `size` region at (tx, ty) is significant.
    pub fn region_significant(
        &self,
        block: &BitPlaneBits,
        tx: usize,
        ty: usize,
        size: usize,
    ) -> bool {
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
pub(crate) fn band_origin(band: usize, scale: usize) -> (usize, usize) {
    let x = if band >= 1 { 1 } else { 0 };
    let y = if band != 1 { 1 } else { 0 };
    (x * scale, y * scale)
}

/// Top-left corner of grandchild group `group` (0..4) inside `band`.
pub(crate) fn grand_child_origin(band: usize, group: usize) -> (usize, usize) {
    let (bx, by) = band_origin(band, 4);
    let gx = if group >= 2 { 2 } else { 0 };
    (bx + gx, by + (group % 2) * 2)
}

/// Append one bit to the symbol value.
pub(crate) fn push_symbol_bit(sym: &mut SymbolDetails, one: bool) {
    sym.sym_len += 1;
    sym.sym_val <<= 1;
    if one {
        sym.sym_val += 1;
    }
}

/// Append the sign of `val` to the symbol sign field.
pub(crate) fn push_symbol_sign(sym: &mut SymbolDetails, val: i32) {
    sym.sign <<= 1;
    sym.sign += sign_of(val);
}

/// Advance to the next symbol slot if the current one holds any bits.
pub(crate) fn advance_if_used(block: &BitPlaneBits, si: &mut usize) {
    if block.symbols_block[*si].sym_len != 0 {
        *si += 1;
    }
}

pub(crate) fn append_parent_refine(block: &mut BitPlaneBits, bit: u8) {
    let rp = &mut block.refine_bits.refine_parent;
    rp.parent_ref_symbol = (rp.parent_ref_symbol << 1) + bit;
    rp.parent_symbol_length += 1;
}

pub(crate) fn append_children_refine(block: &mut BitPlaneBits, bit: u16) {
    let rc = &mut block.refine_bits.refine_children;
    rc.children_ref_symbol = (rc.children_ref_symbol << 1) + bit;
    rc.children_symbol_length += 1;
}

pub(crate) fn append_grand_children_refine(block: &mut BitPlaneBits, band: usize, bit: u16) {
    let rg = &mut block.refine_bits.refine_grand_children[band];
    rg.grand_children_ref_symbol = (rg.grand_children_ref_symbol << 1) + bit;
    rg.grand_children_symbol_length += 1;
}
