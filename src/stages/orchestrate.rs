//! Stage orchestration: run gaggles1 -> gaggles2 -> gaggles3 -> refine.
//!
//! Encode: three passes over all gaggles (stage1 then 2 then 3), then refine bits.
//! Decode: same order, with rate-stop early exit after each pass.

use crate::error::BpeResult;
use crate::pattern::options::coding_options;
use crate::types::{BitPlaneBits, CodingPara};

use super::common::gaggle_ranges;
use super::gaggles1::{stages_de_coding_gaggles1, stages_en_coding_gaggles1};
use super::gaggles2::{stages_de_coding_gaggles2, stages_en_coding_gaggles2};
use super::gaggles3::{stages_de_coding_gaggles3, stages_en_coding_gaggles3};
use super::refine::{ref_bits_de, ref_bits_en};

/// Encode stages 1/2/3 across gaggles, then refinement bits.
pub fn stages_en_coding(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    let s = coding.header.part3.s_20bits as usize;
    let ranges: Vec<(usize, usize, usize)> = gaggle_ranges(s).collect();
    let total_gaggles = ranges.len();

    let mut code_options_all_gaggles: Vec<[u8; 3]> = vec![[0u8; 3]; total_gaggles];
    let mut option_hit_flag: Vec<[bool; 3]> = vec![[false; 3]; total_gaggles];

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        coding_options(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle,
            &mut code_options_all_gaggles[gaggle_index],
        )?;

        stages_en_coding_gaggles1(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;
    }

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        stages_en_coding_gaggles2(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;
    }

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        stages_en_coding_gaggles3(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;
    }

    ref_bits_en(block_info, coding)?;
    Ok(())
}

/// Decode stages 1/2/3 across gaggles, then refinement bits (with rate-stop exits).
pub fn stages_de_coding(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    let s = coding.header.part3.s_20bits as usize;
    let ranges: Vec<(usize, usize, usize)> = gaggle_ranges(s).collect();
    let total_gaggles = ranges.len();

    let mut code_options_all_gaggles: Vec<[u8; 3]> = vec![[0u8; 3]; total_gaggles];
    let mut option_hit_flag: Vec<[bool; 3]> = vec![[false; 3]; total_gaggles];
    coding.decoding_stop_locations.block_no_stop_decoding = 0;

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        stages_de_coding_gaggles1(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &mut code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;

        if coding.decoding_stop_locations.bit_plane_stop_decoding != -1 && coding.rate_reached {
            coding.decoding_stop_locations.block_no_stop_decoding += (gaggle_index as i32) * 16;
            coding.decoding_stop_locations.stopped_stage = 1;
            return Ok(());
        }
    }

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        stages_de_coding_gaggles2(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &mut code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;

        if coding.decoding_stop_locations.bit_plane_stop_decoding != -1 && coding.rate_reached {
            coding.decoding_stop_locations.block_no_stop_decoding += (gaggle_index as i32) * 16;
            coding.decoding_stop_locations.stopped_stage = 2;
            return Ok(());
        }
    }

    if coding.rate_reached {
        coding.decoding_stop_locations.stopped_stage = 2;
        return Ok(());
    }

    for &(gaggle_index, block_start_index, blocks_in_gaggle) in &ranges {
        stages_de_coding_gaggles3(
            coding,
            &mut block_info[block_start_index..],
            blocks_in_gaggle as u8,
            &mut code_options_all_gaggles[gaggle_index],
            &mut option_hit_flag[gaggle_index],
        )?;

        if coding.decoding_stop_locations.bit_plane_stop_decoding != -1 && coding.rate_reached {
            coding.decoding_stop_locations.block_no_stop_decoding += (gaggle_index as i32) * 16;
            coding.decoding_stop_locations.stopped_stage = 3;
            return Ok(());
        }
    }

    if coding.rate_reached {
        coding.decoding_stop_locations.stopped_stage = 3;
        return Ok(());
    }

    ref_bits_de(coding, block_info)?;

    if coding.rate_reached {
        coding.decoding_stop_locations.stopped_stage = 4;
        return Ok(());
    }

    Ok(())
}
