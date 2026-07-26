//! Orchestration: per-block bit-plane scan entry (`block_scan_encode`).

use crate::block::common::{advance_if_used, ScanCtx};
use crate::block::tran_b::scan_tran_b;
use crate::block::tran_d::scan_tran_d;
use crate::block::tran_gi::scan_tran_gi;
use crate::block::tran_hi::scan_tran_hi;
use crate::block::type_ci::scan_type_ci;
use crate::block::type_hij::scan_type_hij;
use crate::block::type_p::scan_type_p;
use crate::error::BpeResult;
use crate::types::{BitPlaneBits, CodingPara};

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
