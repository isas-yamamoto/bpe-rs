use crate::adjust::common::{bump, refine_amount};
use crate::types::{BitPlaneBits, StopLocation, BLOCK_SIZE};

/// `stoppedstage == 1`: decoding stopped while scanning parents;
/// all children/grandchildren get flat `beta_2`.
pub(crate) fn stage1(
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
