//! Stage 1 encode/decode: TypeP symbols -- original/source/StagesCodingGaggles.c

use super::common::{
    emit_code_option_once, mark_stop_at, rate_stop_pending, read_option_and_rice,
    rice_then_signs_then_reset,
};
use crate::bitstream::bits_read;
use crate::error::{BpeError, BpeResult};
use crate::pattern::de_mapping_pattern;
use crate::types::{
    BitPlaneBits, CodingPara, SymbolDetails, ENUM_TYPE_P, INTEGER_WAVELET, MAX_SYMBOLS_IN_BLOCK,
    NEGATIVE_SIGN,
};

/// Encode TypeP symbols across gaggles.
pub(super) fn stages_en_coding_gaggles1(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggles: u8,
    option: &[u8; 3],
    flag_code_option_output: &mut [bool; 3],
) -> BpeResult<()> {
    for block_seq in 0..blocks_in_gaggles as usize {
        if block_info[block_seq].bit_max_ac < coding.bit_plane as u16 {
            continue;
        }
        for symbol_index in 0..MAX_SYMBOLS_IN_BLOCK {
            if block_info[block_seq].symbols_block[symbol_index].type_ == ENUM_TYPE_P {
                let sym_len = block_info[block_seq].symbols_block[symbol_index].sym_len;
                match sym_len {
                    1 | 2 | 3 => {
                        emit_code_option_once(coding, flag_code_option_output, option, sym_len)?;
                        rice_then_signs_then_reset(
                            coding,
                            &mut block_info[block_seq].symbols_block[symbol_index],
                            option,
                            true,
                        )?;
                    }
                    _ => return Err(BpeError::StageCodingError),
                }
            }
        }
    }
    if coding.header.part1.part2_flag && coding.header.part2.stage_stop_2bits == 0 {
        return Ok(());
    }
    Ok(())
}

/// Decode TypeP symbols across gaggles.
pub(super) fn stages_de_coding_gaggles1(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggles: u8,
    code_options_all_gaggles: &mut [u8; 3],
    flag_code_option_output: &mut [bool; 3],
) -> BpeResult<()> {
    let bit_plane = coding.bit_plane;
    let integer_wavelet = coding.header.part4.dwt_type == INTEGER_WAVELET;
    let hl3 = coding.header.part4.custom_wt_hl3;
    let lh3 = coding.header.part4.custom_wt_lh3;
    let hh3 = coding.header.part4.custom_wt_hh3;

    for block_seq in 0..blocks_in_gaggles as usize {
        if block_info[block_seq].bit_max_ac < bit_plane as u16 {
            continue;
        }
        let mut counter: u8 = 0;
        let mut ref_counter: u8 = 0;
        for i in 0..3usize {
            if integer_wavelet
                && ((i == 0 && hl3 >= bit_plane)
                    || (i == 1 && lh3 >= bit_plane)
                    || (i == 2 && hh3 >= bit_plane))
            {
                continue;
            }
            ref_counter += 1;
            if (block_info[block_seq].str_plane_hit_history.type_p & (1 << (2 - i))) == 0 {
                counter += 1;
            }
        }

        if ref_counter != 0 {
            if counter != 0 {
                let (temp_word, stop) = read_option_and_rice(
                    coding,
                    flag_code_option_output,
                    code_options_all_gaggles,
                    counter,
                )?;
                if stop {
                    mark_stop_at(coding, block_seq as i32, 0, 1);
                    return Ok(());
                }

                let mut sym = SymbolDetails::default();
                sym.sym_mapped_pattern = temp_word as u8;
                sym.sym_len = counter;
                sym.type_ = ENUM_TYPE_P;
                de_mapping_pattern(&mut sym)?;

                let mut counter_left = counter;
                for i in 0..3usize {
                    if integer_wavelet
                        && ((i == 0 && hl3 >= bit_plane)
                            || (i == 1 && lh3 >= bit_plane)
                            || (i == 2 && hh3 >= bit_plane))
                    {
                        continue;
                    }
                    let temp_x = if i >= 1 { 1usize } else { 0 };
                    let temp_y = if i != 1 { 1usize } else { 0 };
                    if (block_info[block_seq].str_plane_hit_history.type_p & (1 << (2 - i))) == 0 {
                        let bit = (sym.sym_val & (1 << (counter_left - 1))) > 0;
                        counter_left -= 1;
                        if bit {
                            block_info[block_seq].block_int[temp_x][temp_y] += 1 << (bit_plane - 1);
                            block_info[block_seq].str_plane_hit_history.type_p += 1 << (2 - i);
                            let sign_bit = bits_read(coding, 1)?;
                            if sign_bit == NEGATIVE_SIGN as u32 {
                                block_info[block_seq].block_int[temp_x][temp_y] =
                                    -block_info[block_seq].block_int[temp_x][temp_y];
                            }
                            if rate_stop_pending(coding) {
                                mark_stop_at(coding, block_seq as i32, temp_x as i8, temp_y as i8);
                                return Ok(());
                            }
                        }
                    } else {
                        block_info[block_seq]
                            .refine_bits
                            .refine_parent
                            .parent_ref_symbol += 1 << (2 - i);
                        block_info[block_seq]
                            .refine_bits
                            .refine_parent
                            .parent_symbol_length += 1;
                    }

                    if rate_stop_pending(coding) {
                        mark_stop_at(coding, block_seq as i32, temp_x as i8, temp_y as i8);
                        return Ok(());
                    }
                }
            } else {
                block_info[block_seq]
                    .refine_bits
                    .refine_parent
                    .parent_symbol_length = ref_counter;
                block_info[block_seq]
                    .refine_bits
                    .refine_parent
                    .parent_ref_symbol = 0;
                if integer_wavelet {
                    if hl3 < bit_plane {
                        block_info[block_seq]
                            .refine_bits
                            .refine_parent
                            .parent_ref_symbol += 0x4;
                    }
                    if lh3 < bit_plane {
                        block_info[block_seq]
                            .refine_bits
                            .refine_parent
                            .parent_ref_symbol += 0x2;
                    }
                    if hh3 < bit_plane {
                        block_info[block_seq]
                            .refine_bits
                            .refine_parent
                            .parent_ref_symbol += 0x1;
                    }
                } else {
                    block_info[block_seq]
                        .refine_bits
                        .refine_parent
                        .parent_ref_symbol = 0x7;
                }
            }
        }
    }
    Ok(())
}
