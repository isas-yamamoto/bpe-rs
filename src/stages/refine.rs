//! Refinement-bit stage - moved from original/source/PatternCoding.c (RefBitsEn / RefBitsDe).
//!
//! After stage symbols: encode/decode parent, children, grandchildren refinement bits.

use super::common::{apply_refine_delta, mark_stop_at, rate_stop_pending};
use crate::bitstream::{bits_read, bits_write};
use crate::error::BpeResult;
use crate::types::{BitPlaneBits, CodingPara};

pub(super) fn ref_bits_de(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let bit_plane = coding.bit_plane;
    coding.decoding_stop_locations.block_no_stop_decoding = 0;

    let s = coding.header.part3.s_20bits as usize;

    'block_loop: for block_seq in 0..s {
        if block_info[block_seq].bit_max_ac < bit_plane as u16 {
            continue 'block_loop;
        }

        if block_info[block_seq]
            .refine_bits
            .refine_parent
            .parent_symbol_length
            > 0
        {
            if coding.segment_full || coding.rate_reached {
                return Ok(());
            }
            for i in 0..3usize {
                let mut temp_x: u8 = 0;
                let mut temp_y: u8 = 0;
                if (block_info[block_seq]
                    .refine_bits
                    .refine_parent
                    .parent_ref_symbol
                    & (1 << (2 - i)))
                    > 0
                {
                    let code = bits_read(coding, 1)? as u8;
                    block_info[block_seq]
                        .refine_bits
                        .refine_parent
                        .parent_symbol_length -= 1;
                    temp_x = if i >= 1 { 1 } else { 0 };
                    temp_y = if i != 1 { 1 } else { 0 };
                    if code > 0 {
                        let val =
                            &mut block_info[block_seq].block_int[temp_x as usize][temp_y as usize];
                        apply_refine_delta(val, bit_plane);
                    }
                }
                if rate_stop_pending(coding) {
                    mark_stop_at(coding, block_seq as i32, temp_x as i8, temp_y as i8);
                    return Ok(());
                }
            }
            block_info[block_seq]
                .refine_bits
                .refine_parent
                .parent_ref_symbol = 0;
            block_info[block_seq]
                .refine_bits
                .refine_parent
                .parent_symbol_length = 0;
        }

        if block_info[block_seq]
            .refine_bits
            .refine_children
            .children_symbol_length
            > 0
        {
            if coding.segment_full || coding.rate_reached {
                break 'block_loop;
            }
            for k in 0..3usize {
                let mut counter: i32 = 3;
                let temp_x = (if k >= 1 { 1 } else { 0 }) * 2;
                let temp_y = (if k != 1 { 1 } else { 0 }) * 2;

                for i in temp_x..temp_x + 2 {
                    for j in temp_y..temp_y + 2 {
                        if (block_info[block_seq]
                            .refine_bits
                            .refine_children
                            .children_ref_symbol
                            & (1 << (8 - k as i32 * 4 + counter)))
                            > 0
                        {
                            let code = bits_read(coding, 1)? as u8;
                            block_info[block_seq]
                                .refine_bits
                                .refine_children
                                .children_symbol_length -= 1;
                            if code > 0 {
                                let val = &mut block_info[block_seq].block_int[i][j];
                                apply_refine_delta(val, bit_plane);
                            }
                        }
                        counter -= 1;
                        if rate_stop_pending(coding) {
                            mark_stop_at(coding, block_seq as i32, i as i8, j as i8);
                            return Ok(());
                        }
                    }
                }
            }
            block_info[block_seq]
                .refine_bits
                .refine_children
                .children_ref_symbol = 0;
            block_info[block_seq]
                .refine_bits
                .refine_children
                .children_symbol_length = 0;
        }

        for i in 0..3usize {
            if block_info[block_seq].refine_bits.refine_grand_children[i]
                .grand_children_symbol_length
                > 0
            {
                if coding.segment_full || coding.rate_reached {
                    break;
                }
                for j in 0..4usize {
                    let temp_x =
                        (if i >= 1 { 1 } else { 0 }) * 4 + (if j >= 2 { 1 } else { 0 }) * 2;
                    let temp_y = (if i != 1 { 1 } else { 0 }) * 4 + (j % 2) * 2;
                    let mut counter: i32 = 3;

                    for k in temp_x..temp_x + 2 {
                        for p in temp_y..temp_y + 2 {
                            if (block_info[block_seq].refine_bits.refine_grand_children[i]
                                .grand_children_ref_symbol
                                & (1 << (12 - j as i32 * 4 + counter)))
                                > 0
                            {
                                let code = bits_read(coding, 1)? as u8;
                                block_info[block_seq].refine_bits.refine_grand_children[i]
                                    .grand_children_symbol_length -= 1;
                                if code > 0 {
                                    let val = &mut block_info[block_seq].block_int[k][p];
                                    apply_refine_delta(val, bit_plane);
                                }
                            }
                            counter -= 1;
                            if rate_stop_pending(coding) {
                                mark_stop_at(coding, block_seq as i32, k as i8, p as i8);
                                return Ok(());
                            }
                        }
                    }
                }
            }
            block_info[block_seq].refine_bits.refine_grand_children[i].grand_children_ref_symbol =
                0;
            block_info[block_seq].refine_bits.refine_grand_children[i]
                .grand_children_symbol_length = 0;
        }

        if coding.decoding_stop_locations.bit_plane_stop_decoding != -1 && coding.rate_reached {
            coding.decoding_stop_locations.block_no_stop_decoding = block_seq as i32;
        }
    }
    Ok(())
}

pub(super) fn ref_bits_en(
    block_info: &mut [BitPlaneBits],
    coding: &mut CodingPara,
) -> BpeResult<()> {
    let s = coding.header.part3.s_20bits as usize;
    for i in 0..s {
        if block_info[i].refine_bits.refine_parent.parent_symbol_length > 0 {
            bits_write(
                coding,
                block_info[i].refine_bits.refine_parent.parent_ref_symbol as u32,
                block_info[i].refine_bits.refine_parent.parent_symbol_length as i32,
            )?;
            block_info[i].refine_bits.refine_parent.parent_ref_symbol = 0;
            block_info[i].refine_bits.refine_parent.parent_symbol_length = 0;
        }

        if block_info[i]
            .refine_bits
            .refine_children
            .children_symbol_length
            > 0
        {
            bits_write(
                coding,
                block_info[i]
                    .refine_bits
                    .refine_children
                    .children_ref_symbol as u32,
                block_info[i]
                    .refine_bits
                    .refine_children
                    .children_symbol_length as i32,
            )?;
            block_info[i]
                .refine_bits
                .refine_children
                .children_ref_symbol = 0;
            block_info[i]
                .refine_bits
                .refine_children
                .children_symbol_length = 0;
        }

        for j in 0..3usize {
            if block_info[i].refine_bits.refine_grand_children[j].grand_children_symbol_length > 0 {
                bits_write(
                    coding,
                    block_info[i].refine_bits.refine_grand_children[j].grand_children_ref_symbol
                        as u32,
                    block_info[i].refine_bits.refine_grand_children[j].grand_children_symbol_length
                        as i32,
                )?;
            }
            block_info[i].refine_bits.refine_grand_children[j].grand_children_ref_symbol = 0;
            block_info[i].refine_bits.refine_grand_children[j].grand_children_symbol_length = 0;
        }
    }
    Ok(())
}
