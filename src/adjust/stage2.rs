use crate::adjust::common::{bump, leaf3_refine_then_flat2, refine_amount};
use crate::types::{BitPlaneBits, StopLocation, BLOCK_SIZE};

/// `stoppedstage == 2`: decoding stopped while scanning children;
/// grandchildren get flat `beta_2`, parents get `refine_amount`.
pub(crate) fn stage2(
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
