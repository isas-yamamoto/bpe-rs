//! Stage 2 encode/decode: TranB + TranD + TypeCi symbols -- original/source/StagesCodingGaggles.c

use super::common::{
    emit_code_option_once, mark_stop_at, rate_stop_pending, read_option_and_rice,
    rice_then_signs_then_reset, set_trand_stop,
};
use crate::bitstream::bits_read;
use crate::error::BpeResult;
use crate::pattern::de_mapping_pattern;
use crate::types::{
    BitPlaneBits, CodingPara, SymbolDetails, ENUM_TRAN_B, ENUM_TRAN_D, ENUM_TYPE_CI,
    INTEGER_WAVELET, MAX_SYMBOLS_IN_BLOCK, NEGATIVE_SIGN,
};

/// Encode TranB + TranD + TypeCi symbols across gaggles.
pub(super) fn stages_en_coding_gaggles2(
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
            let type_ = block_info[block_seq].symbols_block[symbol_index].type_;
            if type_ == ENUM_TRAN_B || type_ == ENUM_TRAN_D || type_ == ENUM_TYPE_CI {
                let sym_len = block_info[block_seq].symbols_block[symbol_index].sym_len;
                emit_code_option_once(coding, flag_code_option_output, option, sym_len)?;
                rice_then_signs_then_reset(
                    coding,
                    &mut block_info[block_seq].symbols_block[symbol_index],
                    option,
                    type_ == ENUM_TYPE_CI,
                )?;
            }
        }
    }
    Ok(())
}

/// Decode TranB + TranD + TypeCi symbols across gaggles.
pub(super) fn stages_de_coding_gaggles2(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggles: u8,
    code_options_all_gaggles: &mut [u8; 3],
    flag_code_option_output: &mut [bool; 3],
) -> BpeResult<()> {
    let bit_plane = coding.bit_plane;
    let integer_wavelet = coding.header.part4.dwt_type == INTEGER_WAVELET;
    let hl1 = coding.header.part4.custom_wt_hl1;
    let lh1 = coding.header.part4.custom_wt_lh1;
    let hh1 = coding.header.part4.custom_wt_hh1;
    let hl2 = coding.header.part4.custom_wt_hl2;
    let lh2 = coding.header.part4.custom_wt_lh2;
    let hh2 = coding.header.part4.custom_wt_hh2;

    for block_seq in 0..blocks_in_gaggles as usize {
        if block_info[block_seq].bit_max_ac < bit_plane as u16 {
            continue;
        }
        let mut flag = false;
        let mut counter: u8 = 0;
        for i in 0..3usize {
            if integer_wavelet
                && ((i == 0 && hl2 >= bit_plane && hl1 >= bit_plane)
                    || (i == 1 && lh2 >= bit_plane && lh1 >= bit_plane)
                    || (i == 2 && hh2 >= bit_plane && hh1 >= bit_plane))
            {
                continue;
            }
            flag = true;
            if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - i))) == 0 {
                counter += 1;
            }
        }

        if rate_stop_pending(coding) {
            mark_stop_at(coding, block_seq as i32, 0, 2);
            return Ok(());
        }

        if !flag {
            continue;
        }

        if block_info[block_seq].str_plane_hit_history.tran_b != 1 {
            let temp_word = bits_read(coding, 1)?;
            block_info[block_seq].str_plane_hit_history.tran_b = temp_word as u8;
            if rate_stop_pending(coding) {
                mark_stop_at(coding, block_seq as i32, 0, 2);
                return Ok(());
            }
        }
        if block_info[block_seq].str_plane_hit_history.tran_b == 0 {
            continue;
        }

        if counter != 0 {
            let (temp_word, stop) = read_option_and_rice(
                coding,
                flag_code_option_output,
                code_options_all_gaggles,
                counter,
            )?;
            if stop {
                set_trand_stop(coding, block_info, block_seq);
                return Ok(());
            }

            let mut sym = SymbolDetails::default();
            sym.sym_mapped_pattern = temp_word as u8;
            sym.sym_len = counter;
            sym.type_ = ENUM_TRAN_D;
            de_mapping_pattern(&mut sym)?;

            let mut counter_left = counter;
            for i in 0..3usize {
                if integer_wavelet
                    && ((i == 0 && hl2 >= bit_plane && hl1 >= bit_plane)
                        || (i == 1 && lh2 >= bit_plane && lh1 >= bit_plane)
                        || (i == 2 && hh2 >= bit_plane && hh1 >= bit_plane))
                {
                    continue;
                }
                if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - i))) > 0 {
                    continue;
                }
                block_info[block_seq].str_plane_hit_history.tran_d +=
                    ((sym.sym_val >> (counter_left - 1)) & 0x01) << (2 - i);
                counter_left -= 1;
            }
        }

        for k in 0..3usize {
            block_info[block_seq]
                .refine_bits
                .refine_children
                .children_ref_symbol <<= 4;

            if integer_wavelet
                && ((k == 0 && hl2 >= bit_plane)
                    || (k == 1 && lh2 >= bit_plane)
                    || (k == 2 && hh2 >= bit_plane))
            {
                continue;
            }
            if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - k))) > 0 {
                let mut counter2: u8 = 0;
                for i in 0..4usize {
                    if (block_info[block_seq].str_plane_hit_history.type_ci[k].type_c
                        & (1 << (3 - i)))
                        == 0
                    {
                        counter2 += 1;
                    }
                }
                if counter2 != 0 {
                    let (temp_word, stop) = read_option_and_rice(
                        coding,
                        flag_code_option_output,
                        code_options_all_gaggles,
                        counter2,
                    )?;
                    if stop {
                        set_trand_stop(coding, block_info, block_seq);
                        return Ok(());
                    }

                    let mut sym = SymbolDetails::default();
                    sym.sym_mapped_pattern = temp_word as u8;
                    sym.sym_len = counter2;
                    sym.type_ = ENUM_TYPE_CI;
                    de_mapping_pattern(&mut sym)?;

                    let temp_x = (if k >= 1 { 1usize } else { 0 }) * 2;
                    let temp_y = (if k != 1 { 1usize } else { 0 }) * 2;
                    let mut t: u8 = 0;
                    let mut counter_left = counter2;
                    for i in temp_x..temp_x + 2 {
                        for p in temp_y..temp_y + 2 {
                            if (block_info[block_seq].str_plane_hit_history.type_ci[k].type_c
                                & (1 << (3 - t)))
                                == 0
                            {
                                let bit = (sym.sym_val >> (counter_left - 1)) & 0x01;
                                if bit > 0 {
                                    block_info[block_seq].str_plane_hit_history.type_ci[k]
                                        .type_c += 1 << (3 - t);
                                    block_info[block_seq].block_int[i][p] += 1 << (bit_plane - 1);
                                    let sign_bit = bits_read(coding, 1)?;
                                    if sign_bit == NEGATIVE_SIGN as u32 {
                                        block_info[block_seq].block_int[i][p] =
                                            -block_info[block_seq].block_int[i][p];
                                    }
                                    if rate_stop_pending(coding) {
                                        mark_stop_at(coding, block_seq as i32, i as i8, p as i8);
                                        return Ok(());
                                    }
                                }
                                counter_left -= 1;
                            } else {
                                block_info[block_seq]
                                    .refine_bits
                                    .refine_children
                                    .children_symbol_length += 1;
                                block_info[block_seq]
                                    .refine_bits
                                    .refine_children
                                    .children_ref_symbol += 1 << (3 - t);
                            }
                            t += 1;

                            if rate_stop_pending(coding) {
                                mark_stop_at(coding, block_seq as i32, i as i8, p as i8);
                                return Ok(());
                            }
                        }
                    }
                } else {
                    block_info[block_seq]
                        .refine_bits
                        .refine_children
                        .children_symbol_length += 4;
                    block_info[block_seq]
                        .refine_bits
                        .refine_children
                        .children_ref_symbol += 0xF;
                }
            }
        }
    }
    Ok(())
}
