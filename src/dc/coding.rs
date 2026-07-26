//! DC segment coding — original/source/DC_EnDeCoding.c
//!
//! Encode stages: collect stats -> bit depths -> header -> quantize -> DPCM -> entropy.
//! Decode stages: load blocks -> quantize params -> entropy/DPCM -> extra planes -> dequantize.

use crate::bitstream::{bits_read, bits_write};
use crate::error::BpeResult;
use crate::types::{amplitude, BitPlaneBits, BlockString, CodingPara, BLOCK_SIZE, INTEGER_WAVELET};

use super::dpcm::{dpcm_dc_demapper, dpcm_dc_mapper};
use super::entropy::{dc_entropy_decoder, dc_entropy_encoder};
use super::twos_comp::{conv_twos_comp, deconv_twos_comp};

/// Segment DC/AC statistics collected before bit-depth derivation.
struct SegmentDcAcStats {
    max_ac_segment: i32,
    dc_min: i32,
    dc_max: i32,
}

/// Compute quantization_factor_q_prime from DC/AC bit depths (identical encode/decode).
fn quantization_factor_q_prime(bit_depth_dc: u8, bit_depth_ac: u8) -> i32 {
    if bit_depth_dc <= 3 {
        0
    } else if (bit_depth_dc as i32 - (1 + (bit_depth_ac as i32 >> 1))) <= 1 && bit_depth_dc > 3 {
        bit_depth_dc as i32 - 3
    } else if (bit_depth_dc as i32 - (1 + (bit_depth_ac as i32 >> 1))) > 10 && bit_depth_dc > 3 {
        bit_depth_dc as i32 - 10
    } else {
        1 + (bit_depth_ac as i32 >> 1)
    }
}

/// Copy segment blocks from `block_string` and gather DC/AC amplitude stats.
fn collect_segment_dc_ac_stats(
    coding: &CodingPara,
    block_string: &BlockString,
    block_info: &mut [BitPlaneBits],
) -> SegmentDcAcStats {
    let mut max_ac_segment: i32 = 0;
    let mut dc_min: i32 = 0x10000;
    let mut dc_max: i32 = -0x10000;

    let s = coding.header.part3.s_20bits as usize;

    for block_index in coding.block_counter..(coding.block_counter + s as u32) {
        let index_start = (block_index - coding.block_counter) as usize;
        let base = block_index as usize * BLOCK_SIZE;
        for r in 0..BLOCK_SIZE {
            block_info[index_start].block_int[r] = block_string[base + r];
        }

        let dc_val = block_info[index_start].block_int[0][0];

        if dc_val > dc_max {
            dc_max = dc_val;
        }
        if dc_val < dc_min {
            dc_min = dc_val;
        }

        let mut ac_bit_max_one_block: i32 = 0;
        for k in 0..BLOCK_SIZE {
            for p in 0..BLOCK_SIZE {
                if k == 0 && p == 0 {
                    continue;
                }
                let abs_ac = amplitude(block_info[index_start].block_int[k][p]);
                if abs_ac > ac_bit_max_one_block {
                    ac_bit_max_one_block = abs_ac;
                }
                if abs_ac > max_ac_segment {
                    max_ac_segment = abs_ac;
                }
            }
        }
        block_info[index_start].bit_max_ac = 0;
        while ac_bit_max_one_block > 0 {
            ac_bit_max_one_block >>= 1;
            block_info[index_start].bit_max_ac += 1;
        }
    }

    SegmentDcAcStats {
        max_ac_segment,
        dc_min,
        dc_max,
    }
}

fn derive_bit_depth_ac(mut max_ac_segment: i32) -> u8 {
    let mut depth: u8 = 0;
    while max_ac_segment > 0 {
        max_ac_segment >>= 1;
        depth += 1;
    }
    depth
}

fn derive_bit_depth_dc(dc_min: i32, dc_max: i32) -> u8 {
    let mut max_dc: i32;
    if dc_min >= 0 {
        max_dc = dc_max;
    } else if dc_max <= 0 {
        max_dc = dc_min;
    } else if dc_max >= amplitude(dc_min) {
        max_dc = dc_max;
    } else {
        max_dc = dc_min;
    }

    let mut depth: u8 = 0;
    if max_dc >= 0 {
        while max_dc > 0 {
            max_dc >>= 1;
            depth += 1;
        }
    } else {
        let mut temp: u32 = (-max_dc) as u32;
        while temp > 0 {
            temp >>= 1;
            depth += 1;
        }
        if (1i32 << (depth - 1)) == -max_dc {
            depth -= 1;
        }
    }
    depth + 1
}

/// Apply q and two's-complement shift; fill shifted_dc / dc_remainder.
fn apply_dc_quantization(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    s: usize,
    bit_depth_dc: u8,
) -> BpeResult<()> {
    let bit_depth_ac = coding.header.part1.bit_depth_ac_5bits;
    let q_prime = quantization_factor_q_prime(bit_depth_dc, bit_depth_ac);

    if coding.header.part4.dwt_type == INTEGER_WAVELET {
        coding.quantization_factor_q = q_prime.max(coding.header.part4.custom_wt_ll3 as i32) as u8;
    } else {
        coding.quantization_factor_q = q_prime as u8;
    }

    let k_mask: u32 = (1u32 << coding.quantization_factor_q) - 1;

    for i in 0..s {
        let new_num = conv_twos_comp(block_info[i].block_int[0][0], bit_depth_dc as i16)?;
        block_info[i].shifted_dc = new_num >> coding.quantization_factor_q;
        block_info[i].dc_remainder = (new_num & k_mask) as u16;
    }

    coding.n = (coding.header.part1.bit_depth_dc_5bits as i32 - coding.quantization_factor_q as i32)
        .max(1) as u8;
    Ok(())
}

/// Read DC remainder bit-planes when q exceeds AC depth.
fn read_additional_dc_bitplanes(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    s: usize,
) -> BpeResult<()> {
    if coding.quantization_factor_q > coding.header.part1.bit_depth_ac_5bits {
        let numaddbitplanes: i32 = if coding.header.part4.dwt_type == INTEGER_WAVELET {
            coding.quantization_factor_q as i32
                - (coding.header.part1.bit_depth_ac_5bits as i32)
                    .max(coding.header.part4.custom_wt_ll3 as i32)
        } else {
            coding.quantization_factor_q as i32 - coding.header.part1.bit_depth_ac_5bits as i32
        };

        for i in 0..numaddbitplanes {
            for k in 0..s {
                let temp_word = bits_read(coding, 1)?;
                block_info[k].decoding_dc_remainder +=
                    (temp_word << (coding.quantization_factor_q as i32 - i - 1)) as f32;
            }
        }
    }
    Ok(())
}

/// Shift quantized DC back and convert from two's complement into block_int[0][0].
fn dequantize_dc(
    coding: &mut CodingPara,
    block_info: &mut [BitPlaneBits],
    s: usize,
    bit_depth_dc: u8,
) -> BpeResult<()> {
    coding.n = bit_depth_dc - coding.quantization_factor_q;

    for i in 0..s {
        block_info[i].shifted_dc <<= coding.quantization_factor_q;
        block_info[i].block_int[0][0] =
            deconv_twos_comp(block_info[i].shifted_dc, bit_depth_dc as i16)?;
    }
    Ok(())
}

/// Encode DC for one segment.
pub fn dc_encoding(
    coding: &mut CodingPara,
    block_string: &BlockString,
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let s = coding.header.part3.s_20bits as usize;
    let stats = collect_segment_dc_ac_stats(coding, block_string, block_info);

    coding.header.part1.bit_depth_ac_5bits = derive_bit_depth_ac(stats.max_ac_segment);
    coding.header.part1.bit_depth_dc_5bits = derive_bit_depth_dc(stats.dc_min, stats.dc_max);

    let bit_depth_dc = coding.header.part1.bit_depth_dc_5bits;

    crate::header::header_output(coding)?;

    // C returns early (void) when SegmentFull after header; continue so AC is skipped.
    if coding.segment_full {
        return Ok(());
    }

    apply_dc_quantization(coding, block_info, s, bit_depth_dc)?;

    if coding.n == 1 {
        for i in 0..s {
            bits_write(coding, block_info[i].shifted_dc, 1)?;
        }
        return Ok(());
    }

    dpcm_dc_mapper(block_info, s, coding.n as i16);
    dc_entropy_encoder(coding, block_info)?;

    Ok(())
}

/// Decode DC for one segment.
pub fn dc_decoding(
    coding: &mut CodingPara,
    freq_block_string: &BlockString,
    floating_block_string: &[[f32; BLOCK_SIZE]],
    block_info: &mut [BitPlaneBits],
) -> BpeResult<()> {
    let bit_depth_dc = coding.header.part1.bit_depth_dc_5bits;
    let bit_depth_ac = coding.header.part1.bit_depth_ac_5bits;
    let s = coding.header.part3.s_20bits as usize;

    for i in 0..s {
        for r in 0..BLOCK_SIZE {
            block_info[i].block_int[r] = freq_block_string[i * BLOCK_SIZE + r];
            block_info[i].block_float[r] = floating_block_string[i * BLOCK_SIZE + r];
        }
        block_info[i].decoding_dc_remainder = 0.0;
    }

    let q_prime = quantization_factor_q_prime(bit_depth_dc, bit_depth_ac);

    if coding.header.part4.dwt_type == INTEGER_WAVELET {
        coding.quantization_factor_q = q_prime.max(coding.header.part4.custom_wt_ll3 as i32) as u8;
    } else {
        coding.quantization_factor_q = q_prime as u8;
    }

    coding.n = (bit_depth_dc as i32 - coding.quantization_factor_q as i32).max(1) as u8;

    if coding.n == 1 {
        for i in 0..s {
            block_info[i].shifted_dc = bits_read(coding, 1)?;
        }
    } else {
        dc_entropy_decoder(coding, block_info)?;
        dpcm_dc_demapper(block_info, s, coding.n as i16);
    }

    read_additional_dc_bitplanes(coding, block_info, s)?;
    dequantize_dc(coding, block_info, s, bit_depth_dc)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_bit_depth_counts_significant_bits() {
        assert_eq!(derive_bit_depth_ac(0), 0);
        assert_eq!(derive_bit_depth_ac(1), 1);
        assert_eq!(derive_bit_depth_ac(2), 2);
        assert_eq!(derive_bit_depth_ac(3), 2);
        assert_eq!(derive_bit_depth_ac(4), 3);
        assert_eq!(derive_bit_depth_ac(255), 8);
    }

    #[test]
    fn dc_bit_depth_reserves_a_sign_bit() {
        assert_eq!(derive_bit_depth_dc(0, 0), 1);
        assert_eq!(derive_bit_depth_dc(0, 1), 2);
        assert_eq!(derive_bit_depth_dc(0, 255), 9);
    }

    #[test]
    fn dc_bit_depth_uses_the_dominant_magnitude() {
        // Negative powers of two need one bit less than their positive twin.
        assert_eq!(derive_bit_depth_dc(-8, 0), 4);
        assert_eq!(derive_bit_depth_dc(-9, 0), 5);
        // The larger magnitude decides, regardless of its sign.
        assert_eq!(derive_bit_depth_dc(-4, 100), derive_bit_depth_dc(0, 100));
    }

    #[test]
    fn quantization_factor_is_zero_for_shallow_dc() {
        for depth_ac in 0..=8u8 {
            assert_eq!(quantization_factor_q_prime(3, depth_ac), 0);
            assert_eq!(quantization_factor_q_prime(0, depth_ac), 0);
        }
    }

    #[test]
    fn quantization_factor_follows_the_three_branches() {
        // bit_depth_dc - (1 + bit_depth_ac / 2) <= 1 -> bit_depth_dc - 3
        assert_eq!(quantization_factor_q_prime(5, 8), 2);
        // difference > 10 -> bit_depth_dc - 10
        assert_eq!(quantization_factor_q_prime(12, 0), 2);
        // otherwise 1 + bit_depth_ac / 2
        assert_eq!(quantization_factor_q_prime(8, 6), 4);
    }
}
