use crate::adjust::common::{bump, leaf3_flat1_then_refine, refine_amount};
use crate::types::{BitPlaneBits, StopLocation, BLOCK_SIZE};

/// Final C `else` branch: `stoppedstage` other than 1/2/3 (in practice `4`:
/// decoding stopped while reading refinement bits). By this stage every
/// coefficient's AC significance is already fully known for every block; only
/// the refinement (least significant) bit of the *current* block may be missing
/// for coefficients scanned after the stop position.
pub(crate) fn stage4(
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
