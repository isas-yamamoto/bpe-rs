//! Rate-budget output adjust — original/source/AdjustOutput.c
//!
//! `AdjustOutPut()` runs only on the decoder side, after decoding has stopped
//! early because the rate budget was exhausted. It nudges every coefficient
//! that was not fully refined towards the midpoint of the uncertainty
//! interval implied by the bit-plane / stage / (block, x, y) location where
//! decoding stopped.
//!
//! In the original C, the whole function is duplicated almost verbatim once
//! for `INTEGER_WAVELET` (updating `PtrBlockAddress` as the primary integer
//! array, with `PtrBlockAddressFloating` kept in lock-step) and once more for
//! the floating wavelet case (same branching, `beta_1`/`beta_2`/`BitPlaneCheck`
//! computed with a `- 0.5` bias instead of `- 1`). Line-by-line comparison of
//! the two halves (identical branch conditions, identical comments, only the
//! numeric literal types differ) confirms the branching logic itself is
//! shared, so it is implemented once here as `stage1`..`stage4` and invoked
//! with the appropriate `(beta_1, beta_2, BitPlaneCheck)` triple for either
//! wavelet type. Both the integer and the floating coefficient copies
//! (`block_int` / `block_float`) are always updated together by `bump()`,
//! exactly mirroring the original which updates `PtrBlockAddress` and
//! `PtrBlockAddressFloating` side by side in every branch.

use crate::dc::deconv_twos_comp;
use crate::error::BpeResult;
use crate::types::{BitPlaneBits, CodingPara, StopLocation, BLOCK_SIZE, INTEGER_WAVELET};

/// Adds `amount` to both the integer and the floating copy of coefficient
/// `(m, n)`, following the sign of the *current* value (matches the
/// `if (...>0) += ; else if (...<0) -= ;` pattern repeated throughout the C
/// source for both `PtrBlockAddress` and `PtrBlockAddressFloating`).
#[inline]
fn bump(info: &mut BitPlaneBits, m: usize, n: usize, amount: f32) {
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
fn refine_amount(
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
fn leaf3_refine_then_flat2(
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
fn leaf3_flat1_then_refine(
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

/// `stoppedstage == 1`: decoding stopped while scanning parents;
/// all children/grandchildren get flat `beta_2`.
fn stage1(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    let stop_block = stop.block_no_stop_decoding;
    let x = stop.x_location_stop_decoding as i32;
    let y = stop.y_location_stop_decoding as i32;
    for i in 0..total_blocks {
        for m in 0..BLOCK_SIZE {
            for n in 0..BLOCK_SIZE {
                if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                    continue;
                }
                if m > 1 || n > 1 {
                    // children and grandchildren: not reached at parent stage.
                    bump(&mut blocks[i], m, n, beta_2);
                    continue;
                }
                let ii = i as i32;
                if ii < stop_block {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                } else if ii > stop_block {
                    bump(&mut blocks[i], m, n, beta_2);
                } else if (m as i32) <= x && (n as i32) <= y {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                } else if x == 1 && y == 0 {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                } else {
                    bump(&mut blocks[i], m, n, beta_2);
                }
            }
        }
    }
}

/// `stoppedstage == 2`: decoding stopped while scanning children;
/// grandchildren get flat `beta_2`, parents get `refine_amount`.
fn stage2(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    let stop_block = stop.block_no_stop_decoding;
    let x = stop.x_location_stop_decoding as i32;
    let y = stop.y_location_stop_decoding as i32;
    for i in 0..total_blocks {
        for m in 0..BLOCK_SIZE {
            for n in 0..BLOCK_SIZE {
                if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                    continue;
                }
                if m > 3 || n > 3 {
                    bump(&mut blocks[i], m, n, beta_2);
                    continue;
                }
                if m <= 1 && n <= 1 {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                    continue;
                }
                // children region (parent and grandchildren excluded above).
                let ii = i as i32;
                if ii < stop_block {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                } else if ii > stop_block {
                    bump(&mut blocks[i], m, n, beta_2);
                } else if x < 2 {
                    // upper right block
                    if m >= 2 {
                        bump(&mut blocks[i], m, n, beta_2);
                    } else {
                        leaf3_refine_then_flat2(
                            &mut blocks[i],
                            m,
                            n,
                            x,
                            y,
                            bit_plane_check,
                            beta_1,
                            beta_2,
                        );
                    }
                } else if y < 2 {
                    // lower left block
                    if m < 2 {
                        let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                        bump(&mut blocks[i], m, n, r);
                    } else if n >= 2 {
                        bump(&mut blocks[i], m, n, beta_2);
                    } else {
                        leaf3_refine_then_flat2(
                            &mut blocks[i],
                            m,
                            n,
                            x,
                            y,
                            bit_plane_check,
                            beta_1,
                            beta_2,
                        );
                    }
                } else {
                    // lower right block: every sub-case resolves to refine_amount.
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                }
            }
        }
    }
}

/// `stoppedstage == 3`: decoding stopped while scanning grandchildren.
/// Parents and children (`m<=3 && n<=3`) are always fully decoded by this stage
/// and just get `refine_amount`; the grandchildren region is recursively split
/// first by rows (`m>=4`) then, within the active row-band, by nested 6/2-wide
/// column bands, matching the exact scan order of `BlockScanEncode`'s
/// grandchildren loop.
fn stage3(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    let stop_block = stop.block_no_stop_decoding;
    let x = stop.x_location_stop_decoding as i32;
    let y = stop.y_location_stop_decoding as i32;
    for i in 0..total_blocks {
        for m in 0..BLOCK_SIZE {
            for n in 0..BLOCK_SIZE {
                if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                    continue;
                }
                if m <= 3 && n <= 3 {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                    continue;
                }
                let ii = i as i32;
                if ii < stop_block {
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                } else if ii > stop_block {
                    bump(&mut blocks[i], m, n, beta_2);
                } else {
                    stage3_current_block(
                        &mut blocks[i],
                        m,
                        n,
                        x,
                        y,
                        bit_plane_check,
                        beta_1,
                        beta_2,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stage3_current_block(
    info: &mut BitPlaneBits,
    m: usize,
    n: usize,
    x: i32,
    y: i32,
    bpc: u32,
    b1: f32,
    b2: f32,
) {
    if x < 4 {
        if m >= 4 {
            bump(info, m, n, b2);
        } else if x < 2 && y < 6 {
            if m >= 2 || n >= 6 {
                bump(info, m, n, b2);
            } else {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            }
        } else if x < 2 {
            if m < 2 && n <= 5 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if m >= 2 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if y < 6 {
            if m < 2 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if n >= 6 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if m < 2 || n < 6 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else {
            leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
        }
    } else if y < 4 {
        if m < 4 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else if n >= 4 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else if x < 6 && y < 2 {
            if m >= 6 || n >= 2 {
                bump(info, m, n, b2);
            } else {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            }
        } else if x < 6 {
            if m < 6 && n < 2 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if m >= 6 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if y < 2 {
            if m < 6 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if n >= 2 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if m < 6 || n < 2 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else {
            leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
        }
    } else {
        // x >= 4 && y >= 4
        if m < 4 || n < 4 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else if x < 6 && y < 6 {
            if m >= 6 || n >= 6 {
                bump(info, m, n, b2);
            } else {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            }
        } else if x < 6 {
            if m < 6 && n < 6 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if m >= 6 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if y < 6 {
            if m < 6 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if n >= 6 {
                bump(info, m, n, b2);
            } else {
                leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
            }
        } else if m < 6 || n < 6 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else {
            leaf3_refine_then_flat2(info, m, n, x, y, bpc, b1, b2);
        }
    }
}

/// Final C `else` branch: `stoppedstage` other than 1/2/3 (in practice `4`:
/// decoding stopped while reading refinement bits). By this stage every
/// coefficient's AC significance is already fully known for every block; only
/// the refinement (least significant) bit of the *current* block may be missing
/// for coefficients scanned after the stop position.
fn stage4(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    let stop_block = stop.block_no_stop_decoding;
    let x = stop.x_location_stop_decoding as i32;
    let y = stop.y_location_stop_decoding as i32;
    for i in 0..total_blocks {
        let ii = i as i32;
        if ii < stop_block {
            for m in 0..BLOCK_SIZE {
                for n in 0..BLOCK_SIZE {
                    if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                        continue;
                    }
                    bump(&mut blocks[i], m, n, beta_1);
                }
            }
        } else if ii > stop_block {
            for m in 0..BLOCK_SIZE {
                for n in 0..BLOCK_SIZE {
                    if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                        continue;
                    }
                    let r = refine_amount(&blocks[i], m, n, bit_plane_check, beta_1, beta_2);
                    bump(&mut blocks[i], m, n, r);
                }
            }
        } else {
            for m in 0..BLOCK_SIZE {
                for n in 0..BLOCK_SIZE {
                    if (m == 0 && n == 0) || blocks[i].block_int[m][n] == 0 {
                        continue;
                    }
                    stage4_current_block(
                        &mut blocks[i],
                        m,
                        n,
                        x,
                        y,
                        bit_plane_check,
                        beta_1,
                        beta_2,
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stage4_current_block(
    info: &mut BitPlaneBits,
    m: usize,
    n: usize,
    x: i32,
    y: i32,
    bpc: u32,
    b1: f32,
    b2: f32,
) {
    if x <= 1 && y <= 1 {
        // stop at parent
        if (m as i32) <= x && (n as i32) <= y {
            bump(info, m, n, b1);
        } else {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        }
    } else if x < 4 && y < 4 {
        // stop at children
        if m < 2 && n < 2 {
            bump(info, m, n, b1);
        } else if m > 3 || n > 3 {
            let r = refine_amount(info, m, n, bpc, b1, b2);
            bump(info, m, n, r);
        } else if x < 2 {
            leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
        } else if y < 2 {
            if m < 2 {
                bump(info, m, n, b1);
            } else if n >= 2 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else {
                leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
            }
        } else if m < 2 || n < 2 {
            bump(info, m, n, b1);
        } else {
            leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
        }
    } else {
        // stop at grandchildren
        if m <= 3 && n <= 3 {
            bump(info, m, n, b1);
        } else if x < 4 {
            // upper right grandchildren
            if m >= 4 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if x < 2 && y < 6 {
                if m >= 2 || n >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if x < 2 {
                if m < 2 && n <= 5 {
                    bump(info, m, n, b1);
                } else if m >= 2 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if y < 6 {
                if m < 2 {
                    bump(info, m, n, b1);
                } else if n >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if m < 2 || n < 6 {
                bump(info, m, n, b1);
            } else {
                leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
            }
        } else if y < 4 {
            // lower left grandchildren
            if m < 4 {
                bump(info, m, n, b1);
            } else if n >= 4 {
                let r = refine_amount(info, m, n, bpc, b1, b2);
                bump(info, m, n, r);
            } else if x < 6 && y < 2 {
                if m >= 6 || n >= 2 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if x < 6 {
                if m < 6 && n <= 1 {
                    bump(info, m, n, b1);
                } else if m >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if y < 2 {
                if m < 6 {
                    bump(info, m, n, b1);
                } else if n >= 2 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if m < 6 || n < 2 {
                bump(info, m, n, b1);
            } else {
                leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
            }
        } else {
            // lower right grandchildren
            if m < 4 || n < 4 {
                bump(info, m, n, b1);
            } else if x < 6 && y < 6 {
                if m >= 6 || n >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if x < 6 {
                if m < 6 && n <= 6 {
                    bump(info, m, n, b1);
                } else if m >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if y < 6 {
                if m < 6 {
                    bump(info, m, n, b1);
                } else if n >= 6 {
                    let r = refine_amount(info, m, n, bpc, b1, b2);
                    bump(info, m, n, r);
                } else {
                    leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
                }
            } else if m < 6 || n < 6 {
                bump(info, m, n, b1);
            } else {
                leaf3_flat1_then_refine(info, m, n, x, y, bpc, b1, b2);
            }
        }
    }
}

fn dispatch_stage(
    blocks: &mut [BitPlaneBits],
    total_blocks: usize,
    stop: &StopLocation,
    beta_1: f32,
    beta_2: f32,
    bit_plane_check: u32,
) {
    match stop.stopped_stage {
        1 => stage1(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        2 => stage2(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        3 => stage3(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
        _ => stage4(blocks, total_blocks, stop, beta_1, beta_2, bit_plane_check),
    }
}

pub fn adjust_output(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    let total_blocks = coding.header.part3.s_20bits as usize;

    if coding.header.part4.dwt_type != INTEGER_WAVELET {
        for block in block_info.iter_mut().take(total_blocks) {
            for m in 0..BLOCK_SIZE {
                for n in 0..BLOCK_SIZE {
                    block.block_float[m][n] = block.block_int[m][n] as f32;
                }
            }
        }
    }

    let bit_depth_dc = coding.header.part1.bit_depth_dc_5bits as i16;
    for block in block_info.iter_mut().take(total_blocks) {
        let combined =
            (block.shifted_dc as i32).wrapping_add(block.decoding_dc_remainder as i32) as u32;
        block.block_int[0][0] = deconv_twos_comp(combined, bit_depth_dc)?;
        block.block_float[0][0] = block.block_int[0][0] as f32;
    }

    if coding.rate_reached
        && coding.decoding_stop_locations.block_no_stop_decoding != -1
        && coding.decoding_stop_locations.bit_plane_stop_decoding != -1
    {
        let stop = coding.decoding_stop_locations.clone();

        let b_dc: i32 =
            if (stop.bit_plane_stop_decoding as i32) <= coding.quantization_factor_q as i32 {
                stop.bit_plane_stop_decoding as i32
            } else {
                coding.quantization_factor_q as i32
            };

        if coding.header.part4.dwt_type == INTEGER_WAVELET {
            if b_dc >= 1 {
                let add = 1i32 << (b_dc - 1);
                for block in block_info.iter_mut().take(total_blocks) {
                    block.block_int[0][0] += add;
                }
            }

            let (beta_1, beta_2): (f32, f32) = if stop.bit_plane_stop_decoding >= 1 {
                let bp = stop.bit_plane_stop_decoding as i32;
                (((1i32 << (bp - 1)) - 1) as f32, ((1i32 << bp) - 1) as f32)
            } else {
                (0.0, 0.0)
            };
            let bit_plane_check: u32 = 1u32 << (stop.bit_plane_stop_decoding as u32);

            dispatch_stage(
                block_info,
                total_blocks,
                &stop,
                beta_1,
                beta_2,
                bit_plane_check,
            );
        } else {
            let bit_plane_check: u32 = 1u32 << (stop.bit_plane_stop_decoding as u32);

            if b_dc >= 1 {
                let temp = (1i32 << (b_dc - 1)) as f32 - 0.5;
                for block in block_info.iter_mut().take(total_blocks) {
                    block.block_float[0][0] += temp;
                }
            }

            let (beta_1, beta_2): (f32, f32) = if stop.bit_plane_stop_decoding >= 1 {
                let bp = stop.bit_plane_stop_decoding as i32;
                (
                    ((1i32 << (bp - 1)) as f32) - 0.5,
                    ((1i32 << bp) as f32) - 0.5,
                )
            } else {
                (
                    0.0,
                    if stop.bit_plane_stop_decoding == 0 {
                        0.5
                    } else {
                        0.0
                    },
                )
            };

            dispatch_stage(
                block_info,
                total_blocks,
                &stop,
                beta_1,
                beta_2,
                bit_plane_check,
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HeaderPart1, HeaderPart3, HeaderPart4, StopLocation, BLOCK_SIZE};
    use std::fs;
    use std::path::PathBuf;

    const TOTAL_BLOCKS: usize = 3;
    const BLOCK_NO: i32 = 1;

    // Must match verify/c_unit_tests/gen_adjust_output_vectors.c's int_val/float_val exactly.
    fn int_val(block: i64, m: i64, n: i64, variant: i32) -> i32 {
        let mut v = ((block * 7 + m * 3 + n * 5) % 11 - 5) as i32;
        if variant & 1 != 0 {
            v = -v;
        }
        if variant & 2 != 0 {
            v += ((block * 3 + m * 11 + n * 7) % 7 - 3) as i32;
        }
        v
    }
    fn float_val(block: i64, m: i64, n: i64, variant: i32) -> f32 {
        let mut v = ((block * 5 + m * 7 + n * 2) % 9 - 4) as i32;
        if variant & 1 != 0 {
            v = -v;
        }
        if variant & 2 != 0 {
            v += ((block * 2 + m * 5 + n * 13) % 7 - 3) as i32;
        }
        v as f32
    }

    /// Cross-checks against verify/vectors/adjust_output_vectors.txt, generated
    /// by verify/c_unit_tests/gen_adjust_output_vectors.c, which calls the real
    /// C `AdjustOutPut` directly (not through a full encode/decode roundtrip)
    /// across every (DWTType, stoppedstage, b_DC-branch, X/Y_LocationStopDecoding)
    /// combination -- the full 8x8 X/Y sweep exists specifically to reach the
    /// deep per-stage decision trees that a black-box rate/content sweep can
    /// only hit by chance (see COMPATIBILITY_REPORT.md for why AdjustOutPut
    /// needed this rather than more full-pipeline test cases).
    ///
    /// Ignored by default (like golden_roundtrip.rs) because it needs
    /// verify/run_unit_vectors.py to have generated the vectors file first;
    /// that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../verify/vectors/adjust_output_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, int_csv, float_csv] =
                fields[..]
            else {
                panic!("malformed vector line: {line}");
            };
            let dwt_type: u8 = dwt_type.parse().unwrap();
            let stoppedstage: u8 = stoppedstage.parse().unwrap();
            let b_dc_case: u8 = b_dc_case.parse().unwrap();
            let x_loc: i8 = x_loc.parse().unwrap();
            let y_loc: i8 = y_loc.parse().unwrap();
            let variant: i32 = variant.parse().unwrap();
            let expected_int: Vec<i32> = int_csv.split(',').map(|v| v.parse().unwrap()).collect();
            let expected_float: Vec<f32> =
                float_csv.split(',').map(|v| v.parse().unwrap()).collect();
            assert_eq!(expected_int.len(), TOTAL_BLOCKS * BLOCK_SIZE * BLOCK_SIZE);
            assert_eq!(expected_float.len(), TOTAL_BLOCKS * BLOCK_SIZE * BLOCK_SIZE);

            let mut coding = CodingPara {
                header: crate::types::Header {
                    part1: HeaderPart1 {
                        bit_depth_dc_5bits: 8,
                        ..Default::default()
                    },
                    part3: HeaderPart3 {
                        s_20bits: TOTAL_BLOCKS as u32,
                        ..Default::default()
                    },
                    part4: HeaderPart4 {
                        dwt_type,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                rate_reached: true,
                // b_dc_case 2 makes BitPlaneStopDecoding > QuantizationFactorQ (5 > 2), the
                // only way to reach AdjustOutPut's `b_DC = QuantizationFactorQ` else-branch;
                // must match gen_adjust_output_vectors.c's run_case exactly.
                quantization_factor_q: match b_dc_case {
                    0 => 0,
                    1 => 5,
                    _ => 2,
                },
                decoding_stop_locations: StopLocation {
                    bit_plane_stop_decoding: match b_dc_case {
                        0 => 0,
                        1 => 3,
                        _ => 5,
                    },
                    block_no_stop_decoding: BLOCK_NO,
                    stopped_stage: stoppedstage,
                    x_location_stop_decoding: x_loc,
                    y_location_stop_decoding: y_loc,
                    ..Default::default()
                },
                ..CodingPara::new()
            };

            let mut blocks: Vec<BitPlaneBits> = (0..TOTAL_BLOCKS)
                .map(|b| {
                    let mut block = BitPlaneBits {
                        shifted_dc: (100 + b) as u32,
                        decoding_dc_remainder: 0.0,
                        ..Default::default()
                    };
                    for m in 0..BLOCK_SIZE {
                        for n in 0..BLOCK_SIZE {
                            block.block_int[m][n] = int_val(b as i64, m as i64, n as i64, variant);
                            block.block_float[m][n] =
                                float_val(b as i64, m as i64, n as i64, variant);
                        }
                    }
                    block
                })
                .collect();

            adjust_output(&mut coding, &mut blocks).unwrap();

            let got_int: Vec<i32> = blocks
                .iter()
                .flat_map(|b| b.block_int.iter().flat_map(|row| row.iter().copied()))
                .collect();
            let got_float: Vec<f32> = blocks
                .iter()
                .flat_map(|b| b.block_float.iter().flat_map(|row| row.iter().copied()))
                .collect();

            assert_eq!(
                got_int, expected_int,
                "dwt_type={} stoppedstage={} b_dc={} x={} y={} variant={}: int mismatch: rust={:?} c={:?}",
                dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, got_int, expected_int
            );
            assert_eq!(
                got_float, expected_float,
                "dwt_type={} stoppedstage={} b_dc={} x={} y={} variant={}: float mismatch: rust={:?} c={:?}",
                dwt_type, stoppedstage, b_dc_case, x_loc, y_loc, variant, got_float, expected_float
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }
}
