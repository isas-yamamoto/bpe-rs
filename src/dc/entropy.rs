//! DC entropy (Rice-like) encoding/decoding - original/source/DC_EnDeCoding.c

use crate::bitstream::{bits_read, bits_write};
use crate::error::{BpeError, BpeResult};
use crate::rice::{select_rice_k, UNCODED_FLAG};
use crate::types::{BitPlaneBits, CodingPara, GAGGLE_SIZE, INTEGER_WAVELET};

fn dc_encoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    start_index: usize,
    gaggles: usize,
    max_k: i32,
    id_length: i32,
) -> BpeResult<()> {
    let mapped: Vec<u32> = (0..(start_index + gaggles))
        .map(|i| block_info[i].mapped_dc)
        .collect();
    let min_k = select_rice_k(
        &mapped,
        start_index,
        gaggles,
        coding.n,
        max_k,
        coding.header.part3.opt_dc_select,
    );

    bits_write(coding, min_k as u32, id_length)?;

    for i in start_index..(start_index + gaggles) {
        if (min_k == UNCODED_FLAG) || (i == 0) {
            bits_write(coding, block_info[i].mapped_dc, coding.n as i32)?;
        } else {
            bits_write(coding, 1, ((block_info[i].mapped_dc >> min_k) + 1) as i32)?;
        }
    }
    if min_k != UNCODED_FLAG {
        for i in start_index.max(1)..(start_index + gaggles) {
            bits_write(coding, block_info[i].mapped_dc, min_k)?;
        }
    }
    Ok(())
}

pub fn dc_entropy_encoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let (max_k, id_length) = if coding.n == 2 {
        (0, 1)
    } else if coding.n <= 4 {
        (2, 2)
    } else if coding.n <= 8 {
        (6, 3)
    } else {
        (8, 4)
    };

    let s = coding.header.part3.s_20bits as usize;
    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        dc_encoder(
            coding,
            block_info,
            gaggle_start_index,
            gaggles,
            max_k,
            id_length,
        )?;
        gaggle_start_index += gaggles;
    }

    if coding.header.part1.bit_depth_ac_5bits < coding.quantization_factor_q {
        let numaddbitplanes: i32 = if coding.header.part4.dwt_type == INTEGER_WAVELET {
            coding.quantization_factor_q as i32
                - (coding.header.part1.bit_depth_ac_5bits as i32)
                    .max(coding.header.part4.custom_wt_ll3 as i32)
        } else {
            coding.quantization_factor_q as i32 - coding.header.part1.bit_depth_ac_5bits as i32
        };

        for i in 0..numaddbitplanes {
            for k in 0..s {
                bits_write(
                    coding,
                    (block_info[k].dc_remainder >> (coding.quantization_factor_q as i32 - i - 1))
                        as u32,
                    1,
                )?;
            }
        }
    }
    Ok(())
}

fn dc_gaggle_decoding(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    start_index: usize,
    gaggles: usize,
    id_length: i16,
) -> BpeResult<()> {
    let temp_word = bits_read(coding, id_length)?;
    let min_k = temp_word as u8;

    let uncoded = (id_length == 1 && min_k == 1)
        || (id_length == 2 && min_k == 3)
        || (id_length == 3 && min_k == 7)
        || (id_length == 4 && min_k == 15);

    for i in start_index..(start_index + gaggles) {
        if uncoded || (i == 0) {
            let w = bits_read(coding, coding.n as i16)?;
            block_info[i].mapped_dc = w;
        } else {
            let mut counter: u32 = 0;
            let mut word = bits_read(coding, 1)?;
            while (word == 0) && !coding.rate_reached {
                counter += 1;
                word = bits_read(coding, 1)?;
            }
            if coding.rate_reached {
                break;
            }
            block_info[i].mapped_dc = counter;
            block_info[i].mapped_dc <<= min_k;
        }
    }
    if !uncoded && !coding.rate_reached {
        for i in start_index.max(1)..(start_index + gaggles) {
            let w = bits_read(coding, min_k as i16)?;
            block_info[i].mapped_dc += w;
            if coding.rate_reached {
                break;
            }
        }
    }
    Ok(())
}

pub fn dc_entropy_decoder(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let id_length: i16 = if coding.n == 2 {
        1
    } else if coding.n <= 4 {
        2
    } else if coding.n <= 8 {
        3
    } else if coding.n <= 10 {
        4
    } else {
        return Err(BpeError::DataError);
    };

    let s = coding.header.part3.s_20bits as usize;
    let mut gaggle_start_index: usize = 0;
    while gaggle_start_index < s {
        let gaggles = GAGGLE_SIZE.min(s - gaggle_start_index);
        dc_gaggle_decoding(coding, block_info, gaggle_start_index, gaggles, id_length)?;
        gaggle_start_index += gaggles;
    }
    Ok(())
}
