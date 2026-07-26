//! AC bit-plane coding loop — original/source/AC_BitPlaneCoding.c
//!
//! Pipeline stage: for each AC bit plane (MSB -> LSB):
//!   1. optional DC remainder bit for this plane
//!   2. block_scan_encode (encode) / stages decode (decode)
//!   3. stages_en_coding (encode)

use crate::bitstream::{bits_read, bits_write};
use crate::block::block_scan_encode;
use crate::error::BpeResult;
use crate::stages::{stages_de_coding, stages_en_coding};
use crate::types::{BitPlaneBits, CodingPara, INTEGER_WAVELET};

use super::depth::{ac_depth_decoder, ac_depth_encoder};

/// True when the DC quantization remainder bit is coded on this AC bit plane.
fn dc_remainder_plane_active(coding: &CodingPara, bit_plane: u8) -> bool {
    (bit_plane <= coding.quantization_factor_q)
        && (coding.header.part4.dwt_type != INTEGER_WAVELET
            || (coding.quantization_factor_q > coding.header.part4.custom_wt_ll3
                && coding.header.part4.custom_wt_ll3 < bit_plane))
}

/// Encode one AC bit plane: DC remainder bits, block scan, then stage coding.
fn encode_one_bitplane(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    bit_plane: u8,
    s: usize,
) -> BpeResult<()> {
    if dc_remainder_plane_active(coding, bit_plane) {
        for i in 0..s {
            bits_write(
                coding,
                ((block_info[i].dc_remainder >> (bit_plane - 1)) & 0x01) as u32,
                1,
            )?;
        }
    }

    if coding.segment_full {
        return Ok(());
    }

    block_scan_encode(coding, block_info)?;

    if coding.segment_full {
        return Ok(());
    }

    stages_en_coding(coding, block_info)?;
    Ok(())
}

/// Decode one AC bit plane: DC remainder bits, then stage decoding.
fn decode_one_bitplane(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    bit_plane: u8,
    s: usize,
) -> BpeResult<()> {
    if dc_remainder_plane_active(coding, bit_plane) {
        for i in 0..s {
            if coding.segment_full {
                break;
            }
            let temp_word = bits_read(coding, 1)?;
            block_info[i].decoding_dc_remainder += (temp_word << (bit_plane - 1)) as f32;
        }
    }

    if !coding.segment_full {
        stages_de_coding(coding, block_info)?;
    }

    Ok(())
}

/// AC depth header + per-plane encode loop.
pub fn ac_bpe_encoding(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    if coding.header.part1.bit_depth_ac_5bits != 0 {
        let s = coding.header.part3.s_20bits as usize;

        if coding.header.part1.bit_depth_ac_5bits == 1 {
            for i in 0..s {
                bits_write(coding, block_info[i].bit_max_ac as u32, 1)?;
            }
        } else {
            ac_depth_encoder(coding, block_info)?;
        }

        let mut bit_plane = coding.header.part1.bit_depth_ac_5bits;
        while bit_plane > 0 {
            coding.bit_plane = bit_plane;
            if coding.header.part2.bit_plane_stop_5bits == bit_plane
                && coding.header.part1.part2_flag
            {
                return Ok(());
            }

            encode_one_bitplane(coding, block_info, bit_plane, s)?;

            if coding.segment_full {
                return Ok(());
            }

            bit_plane -= 1;
        }
    }
    Ok(())
}

pub fn check_use_fill(coding: &mut CodingPara) -> BpeResult<()> {
    if coding.header.part2.seg_byte_limit_27bits != 0 && coding.header.part2.use_fill {
        if (coding.header.part2.seg_byte_limit_27bits << 3) < coding.bits.seg_bit_counter {
            let remainder_bits = coding.bits.seg_bit_counter as i64
                - ((coding.header.part2.seg_byte_limit_27bits as i64) << 3);
            let mut remaining = remainder_bits;
            while remaining > 0 {
                bits_read(coding, 1)?;
                if coding.segment_full {
                    return Ok(());
                }
                remaining -= 1;
            }
        }
    }
    Ok(())
}

/// AC depth header + per-plane decode loop.
pub fn ac_bpe_decoding(coding: &mut CodingPara, block_info: &mut [BitPlaneBits]) -> BpeResult<()> {
    if coding.rate_reached {
        return Ok(());
    }

    let s = coding.header.part3.s_20bits as usize;

    if coding.header.part1.bit_depth_ac_5bits != 0 {
        if coding.header.part1.bit_depth_ac_5bits == 1 {
            for i in 0..s {
                let temp_word = bits_read(coding, 1)?;
                block_info[i].bit_max_ac = temp_word as u16;
            }
        } else {
            ac_depth_decoder(coding, block_info)?;
        }
    }
    if coding.segment_full {
        return Ok(());
    }

    let mut bit_plane = coding.header.part1.bit_depth_ac_5bits;
    while bit_plane > 0 {
        if coding.segment_full || coding.rate_reached {
            return Ok(());
        }

        coding.bit_plane = bit_plane;
        decode_one_bitplane(coding, block_info, bit_plane, s)?;

        bit_plane -= 1;
    }
    check_use_fill(coding)?;
    Ok(())
}
