//! Shared coefficient bump / refine helpers (`common`) for AdjustOutPut stages.

use crate::types::BitPlaneBits;

/// Adds `amount` to both the integer and the floating copy of coefficient
/// `(m, n)`, following the sign of the *current* value (matches the
/// `if (...>0) += ; else if (...<0) -= ;` pattern repeated throughout the C
/// source for both `PtrBlockAddress` and `PtrBlockAddressFloating`).
#[inline]
pub(crate) fn bump(info: &mut BitPlaneBits, m: usize, n: usize, amount: f32) {
    if info.block_int[m][n] > 0 {
        info.block_int[m][n] += amount as i32;
    } else if info.block_int[m][n] < 0 {
        info.block_int[m][n] -= amount as i32;
    }
    if info.block_float[m][n] > 0.0 {
        info.block_float[m][n] += amount;
    } else if info.block_float[m][n] < 0.0 {
        info.block_float[m][n] -= amount;
    }
}

/// C idiom
/// `if ((DWORD32) abs(...) > BitPlaneCheck) refinement = beta_2; else refinement = beta_1;`:
/// decides whether coefficient `(m, n)` had already been selected before the
/// current bit-plane (large magnitude -> `beta_2`) or was newly selected at the
/// current bit-plane (`beta_1`).
#[inline]
pub(crate) fn refine_amount(
    info: &BitPlaneBits,
    m: usize,
    n: usize,
    bit_plane_check: u32,
    beta_1: f32,
    beta_2: f32,
) -> f32 {
    if info.block_int[m][n].unsigned_abs() > bit_plane_check {
        beta_2
    } else {
        beta_1
    }
}

/// Leaf 3-way test used at the innermost 2x2 granularity: cells scanned
/// *before* the stop position (`m < x`, or `m == x && n <= y`) use the
/// per-coefficient `refine_amount`; cells scanned *after* use flat `beta_2`
/// (not yet decoded at all).
#[inline]
pub(crate) fn leaf3_refine_then_flat2(
    info: &mut BitPlaneBits,
    m: usize,
    n: usize,
    x: i32,
    y: i32,
    bit_plane_check: u32,
    beta_1: f32,
    beta_2: f32,
) {
    if (m as i32) < x || ((m as i32) == x && (n as i32) <= y) {
        let r = refine_amount(info, m, n, bit_plane_check, beta_1, beta_2);
        bump(info, m, n, r);
    } else {
        bump(info, m, n, beta_2);
    }
}

/// Mirror image of [`leaf3_refine_then_flat2`], used by stage 4 (the
/// refinement-bit stage): cells scanned *before* the stop position already
/// went through one more full refinement step, so they get the flat midpoint
/// `beta_1`; cells scanned *after* get the per-coefficient `refine_amount`.
#[inline]
pub(crate) fn leaf3_flat1_then_refine(
    info: &mut BitPlaneBits,
    m: usize,
    n: usize,
    x: i32,
    y: i32,
    bit_plane_check: u32,
    beta_1: f32,
    beta_2: f32,
) {
    if (m as i32) < x || ((m as i32) == x && (n as i32) <= y) {
        bump(info, m, n, beta_1);
    } else {
        let r = refine_amount(info, m, n, bit_plane_check, beta_1, beta_2);
        bump(info, m, n, r);
    }
}
