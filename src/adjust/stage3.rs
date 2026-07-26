use crate::adjust::common::{bump, leaf3_refine_then_flat2, refine_amount};
use crate::types::{BitPlaneBits, StopLocation, BLOCK_SIZE};

/// `stoppedstage == 3`: decoding stopped while scanning grandchildren.
/// Parents and children (`m<=3 && n<=3`) are always fully decoded by this stage
/// and just get `refine_amount`; the grandchildren region is recursively split
/// first by rows (`m>=4`) then, within the active row-band, by nested 6/2-wide
/// column bands, matching the exact scan order of `BlockScanEncode`'s
/// grandchildren loop.
pub(crate) fn stage3(
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
