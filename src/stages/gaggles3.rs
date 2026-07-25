//! Stage coding gaggles 3: TranGi + TranHi + TypeHij symbols (encode + decode) - original/source/StagesCodingGaggles.c

use crate::bitstream::bits_read;
use crate::error::BpeResult;
use crate::pattern::de_mapping_pattern;
use crate::types::{
    BitPlaneBits, CodingPara, SymbolDetails, ENUM_TRAN_GI, ENUM_TRAN_HI, ENUM_TYPE_HIJ,
    INTEGER_WAVELET, MAX_SYMBOLS_IN_BLOCK, NEGATIVE_SIGN,
};

use super::common::{
    emit_code_option_once, mark_stop_at, rate_stop_pending, read_option_and_rice,
    rice_then_signs_then_reset, set_trangi_stop,
};

/// Encode TranGi + TranHi + TypeHij symbols across gaggles.
pub(super) fn stages_en_coding_gaggles3(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggles: u8,
    option: &[u8; 3],
    flag_code_option_output: &mut [bool; 3],
) -> BpeResult<()> {
    if coding.header.part1.part2_flag && coding.header.part2.stage_stop_2bits == 1 {
        return Ok(());
    }

    for block_seq in 0..blocks_in_gaggles as usize {
        if block_info[block_seq].bit_max_ac < coding.bit_plane as u16 {
            continue;
        }
        for symbol_index in 0..MAX_SYMBOLS_IN_BLOCK {
            let type_ = block_info[block_seq].symbols_block[symbol_index].type_;
            if type_ == ENUM_TRAN_GI || type_ == ENUM_TRAN_HI || type_ == ENUM_TYPE_HIJ {
                let sym_len = block_info[block_seq].symbols_block[symbol_index].sym_len;
                emit_code_option_once(coding, flag_code_option_output, option, sym_len)?;
                rice_then_signs_then_reset(
                    coding,
                    &mut block_info[block_seq].symbols_block[symbol_index],
                    option,
                    type_ == ENUM_TYPE_HIJ,
                )?;
            }
        }
    }
    Ok(())
}

/// Decode TranGi + TranHi + TypeHij symbols across gaggles.
pub(super) fn stages_de_coding_gaggles3(
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

    for block_seq in 0..blocks_in_gaggles as usize {
        if block_info[block_seq].bit_max_ac < bit_plane as u16 {
            continue;
        }
        if block_info[block_seq].str_plane_hit_history.tran_b == 0 {
            continue;
        }

        let mut flag = false;
        let mut counter: u8 = 0;
        if integer_wavelet {
            for k in 0..3usize {
                if (k == 0 && hl1 >= bit_plane)
                    || (k == 1 && lh1 >= bit_plane)
                    || (k == 2 && hh1 >= bit_plane)
                {
                    continue;
                }
                flag = true;
                if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - k))) > 0
                    && (block_info[block_seq].str_plane_hit_history.tran_gi & (1 << (2 - k))) == 0
                {
                    counter += 1;
                }
            }
        } else {
            flag = true;
            for k in 0..3usize {
                if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - k))) > 0
                    && (block_info[block_seq].str_plane_hit_history.tran_gi & (1 << (2 - k))) == 0
                {
                    counter += 1;
                }
            }
        }

        if !flag {
            continue;
        }

        if counter > 0 {
            let (temp_word, stop) = read_option_and_rice(
                coding,
                flag_code_option_output,
                code_options_all_gaggles,
                counter,
            )?;
            if stop {
                set_trangi_stop(coding, block_info, block_seq);
                return Ok(());
            }

            let mut sym = SymbolDetails::default();
            sym.sym_mapped_pattern = temp_word as u8;
            sym.sym_len = counter;
            sym.type_ = ENUM_TRAN_GI;
            de_mapping_pattern(&mut sym)?;

            let mut counter_left = counter;
            for i in 0..3usize {
                if (block_info[block_seq].str_plane_hit_history.tran_d & (1 << (2 - i))) > 0 {
                    if integer_wavelet
                        && ((i == 0 && hl1 >= bit_plane)
                            || (i == 1 && lh1 >= bit_plane)
                            || (i == 2 && hh1 >= bit_plane))
                    {
                        continue;
                    }
                    if (block_info[block_seq].str_plane_hit_history.tran_gi & (1 << (2 - i))) == 0 {
                        block_info[block_seq].str_plane_hit_history.tran_gi +=
                            ((sym.sym_val >> (counter_left - 1)) & 0x01) << (2 - i);
                        counter_left -= 1;
                    }
                }
            }
        }

        for k in 0..3usize {
            if integer_wavelet
                && ((k == 0 && hl1 >= bit_plane)
                    || (k == 1 && lh1 >= bit_plane)
                    || (k == 2 && hh1 >= bit_plane))
            {
                continue;
            }

            if (block_info[block_seq].str_plane_hit_history.tran_gi & (1 << (2 - k))) != 0 {
                let mut counter2: u8 = 0;
                for i in 0..4usize {
                    if (block_info[block_seq].str_plane_hit_history.tran_hi[k].tran_h
                        & (1 << (3 - i)))
                        == 0
                    {
                        counter2 += 1;
                    }
                }
                if counter2 == 0 {
                    continue;
                }

                let (temp_word, stop) = read_option_and_rice(
                    coding,
                    flag_code_option_output,
                    code_options_all_gaggles,
                    counter2,
                )?;
                if stop {
                    set_trangi_stop(coding, block_info, block_seq);
                    return Ok(());
                }

                let mut sym = SymbolDetails::default();
                sym.sym_mapped_pattern = temp_word as u8;
                sym.sym_len = counter2;
                sym.type_ = ENUM_TRAN_HI;
                de_mapping_pattern(&mut sym)?;

                if sym.sym_val > 0 {
                    let mut counter_left = counter2;
                    for i in 0..4usize {
                        if (block_info[block_seq].str_plane_hit_history.tran_hi[k].tran_h
                            & (1 << (3 - i)))
                            != 0
                        {
                            continue;
                        }
                        let bit = (sym.sym_val >> (counter_left - 1)) & 0x01;
                        block_info[block_seq].str_plane_hit_history.tran_hi[k].tran_h +=
                            bit << (3 - i);
                        counter_left -= 1;
                    }
                }
            }
        }

        if rate_stop_pending(coding) {
            mark_stop_at(coding, block_seq as i32, 4, 0);
            return Ok(());
        }

        // TypeHij
        for i in 0..3usize {
            if integer_wavelet
                && ((i == 0 && hl1 >= bit_plane)
                    || (i == 1 && lh1 >= bit_plane)
                    || (i == 2 && hh1 >= bit_plane))
            {
                continue;
            }

            for k in 0..4usize {
                let temp_x =
                    (if i >= 1 { 1usize } else { 0 }) * 4 + (if k >= 2 { 1usize } else { 0 }) * 2;
                let temp_y = (if i != 1 { 1usize } else { 0 }) * 4 + (k % 2) * 2;

                block_info[block_seq].refine_bits.refine_grand_children[i]
                    .grand_children_ref_symbol <<= 4;

                if (block_info[block_seq].str_plane_hit_history.tran_hi[i].tran_h & (1 << (3 - k)))
                    > 0
                {
                    let mut counter3: u8 = 0;
                    for p in 0..4usize {
                        if (block_info[block_seq].str_plane_hit_history.type_hij[i].type_hij[k]
                            .tran_h
                            & (1 << (3 - p)))
                            == 0
                        {
                            counter3 += 1;
                        }
                    }
                    if counter3 != 0 {
                        let (temp_word, stop) = read_option_and_rice(
                            coding,
                            flag_code_option_output,
                            code_options_all_gaggles,
                            counter3,
                        )?;
                        if stop {
                            mark_stop_at(coding, block_seq as i32, temp_x as i8, temp_y as i8);
                            return Ok(());
                        }

                        let mut sym = SymbolDetails::default();
                        sym.sym_mapped_pattern = temp_word as u8;
                        sym.sym_len = counter3;
                        sym.type_ = ENUM_TYPE_HIJ;
                        de_mapping_pattern(&mut sym)?;

                        let mut counter_left = counter3;
                        for p in 0..4usize {
                            if (block_info[block_seq].str_plane_hit_history.type_hij[i].type_hij[k]
                                .tran_h
                                & (1 << (3 - p)))
                                != 0
                            {
                                block_info[block_seq].refine_bits.refine_grand_children[i]
                                    .grand_children_ref_symbol += 1 << (3 - p);
                                block_info[block_seq].refine_bits.refine_grand_children[i]
                                    .grand_children_symbol_length += 1;
                                continue;
                            }
                            let bit = (sym.sym_val & (1 << (counter_left - 1))) > 0;
                            counter_left -= 1;
                            if bit {
                                let rr = temp_x + p / 2;
                                let cc = temp_y + p % 2;
                                block_info[block_seq].block_int[rr][cc] += 1 << (bit_plane - 1);
                                block_info[block_seq].str_plane_hit_history.type_hij[i].type_hij
                                    [k]
                                    .tran_h += 1 << (3 - p);
                                let sign_bit = bits_read(coding, 1)?;
                                if sign_bit == NEGATIVE_SIGN as u32 {
                                    block_info[block_seq].block_int[rr][cc] =
                                        -block_info[block_seq].block_int[rr][cc];
                                }
                                if rate_stop_pending(coding) {
                                    mark_stop_at(coding, block_seq as i32, rr as i8, cc as i8);
                                    return Ok(());
                                }
                            }
                            if rate_stop_pending(coding) {
                                let rr = temp_x + p / 2;
                                let cc = temp_y + p % 2;
                                mark_stop_at(coding, block_seq as i32, rr as i8, cc as i8);
                                return Ok(());
                            }
                        }
                    } else {
                        block_info[block_seq].refine_bits.refine_grand_children[i]
                            .grand_children_ref_symbol += 0xF;
                        block_info[block_seq].refine_bits.refine_grand_children[i]
                            .grand_children_symbol_length += 4;
                    }
                }
            }
        }
    }
    Ok(())
}
