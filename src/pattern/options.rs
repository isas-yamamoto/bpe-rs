//! Rice code-option selection per gaggle - original/source/PatternCoding.c

use crate::error::{BpeError, BpeResult};
use crate::types::{BitPlaneBits, CodingPara, ENUM_NONE, MAX_SYMBOLS_IN_BLOCK};

use super::mapping::pattern_mapping;

/// Important: like C, this mutates each symbol via pattern_mapping so that
/// subsequent Rice coding sees sym_mapped_pattern (BlockScan only sets sym_val).
pub fn coding_options(
    coding: &CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggle: usize,
    option: &mut [u8; 3],
) -> BpeResult<()> {
    let mut bits_counter_2bits: [u32; 2] = [0; 2];
    let mut bits_counter_3bits: [u32; 3] = [0; 3];
    let mut bits_counter_4bits: [u32; 4] = [0; 4];

    for block_seq in 0..blocks_in_gaggle {
        if block_info[block_seq].bit_max_ac < coding.bit_plane as u16 {
            continue;
        }
        for symbol_index in 0..MAX_SYMBOLS_IN_BLOCK {
            if block_info[block_seq].symbols_block[symbol_index].type_ == ENUM_NONE {
                continue;
            }
            // C always PatternMapping before continue/statistics.
            pattern_mapping(&mut block_info[block_seq].symbols_block[symbol_index])?;
            let sym_len = block_info[block_seq].symbols_block[symbol_index].sym_len;
            if sym_len == 1 {
                continue;
            }
            let mapped = block_info[block_seq].symbols_block[symbol_index].sym_mapped_pattern;
            match sym_len {
                2 => {
                    match mapped {
                        0 => bits_counter_2bits[0] += 1,
                        1 => bits_counter_2bits[0] += 2,
                        2 => bits_counter_2bits[0] += 3,
                        3 => bits_counter_2bits[0] += 3,
                        _ => return Err(BpeError::PatternCodingError),
                    }
                    bits_counter_2bits[1] += 2;
                }
                3 => {
                    if mapped <= 2 {
                        bits_counter_3bits[0] += mapped as u32 + 1;
                    } else if mapped <= 5 {
                        bits_counter_3bits[0] += 5;
                    } else if mapped <= 7 {
                        bits_counter_3bits[0] += 6;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 1 {
                        bits_counter_3bits[1] += 2;
                    } else if mapped <= 3 {
                        bits_counter_3bits[1] += 3;
                    } else if mapped <= 7 {
                        bits_counter_3bits[1] += 4;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }
                    bits_counter_3bits[2] += 3;
                }
                4 => {
                    if mapped <= 3 {
                        bits_counter_4bits[0] += mapped as u32 + 1;
                    } else if mapped <= 7 {
                        bits_counter_4bits[0] += 7;
                    } else if mapped <= 15 {
                        bits_counter_4bits[0] += 8;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 1 {
                        bits_counter_4bits[1] += 2;
                    } else if mapped <= 3 {
                        bits_counter_4bits[1] += 3;
                    } else if mapped <= 5 {
                        bits_counter_4bits[1] += 4;
                    } else if mapped <= 11 {
                        bits_counter_4bits[1] += 6;
                    } else if mapped <= 15 {
                        bits_counter_4bits[1] += 7;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 3 {
                        bits_counter_4bits[2] += 3;
                    } else if mapped <= 7 {
                        bits_counter_4bits[2] += 4;
                    } else if mapped <= 15 {
                        bits_counter_4bits[2] += 5;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    bits_counter_4bits[3] += 4;
                }
                _ => {}
            }
        }
    }

    if bits_counter_2bits[0] < bits_counter_2bits[1] {
        option[0] = 0;
    } else {
        option[0] = 1;
    }

    if bits_counter_3bits[2] <= bits_counter_3bits[0]
        && bits_counter_3bits[2] <= bits_counter_3bits[1]
    {
        option[1] = 3;
    } else if bits_counter_3bits[0] <= bits_counter_3bits[1]
        && bits_counter_3bits[0] <= bits_counter_3bits[2]
    {
        option[1] = 0;
    } else if bits_counter_3bits[1] <= bits_counter_3bits[0]
        && bits_counter_3bits[1] <= bits_counter_3bits[2]
    {
        option[1] = 1;
    }

    if bits_counter_4bits[3] <= bits_counter_4bits[1]
        && bits_counter_4bits[3] <= bits_counter_4bits[0]
        && bits_counter_4bits[3] <= bits_counter_4bits[2]
    {
        option[2] = 3;
    } else if bits_counter_4bits[0] <= bits_counter_4bits[1]
        && bits_counter_4bits[0] <= bits_counter_4bits[2]
        && bits_counter_4bits[0] <= bits_counter_4bits[3]
    {
        option[2] = 0;
    } else if bits_counter_4bits[1] <= bits_counter_4bits[0]
        && bits_counter_4bits[1] <= bits_counter_4bits[2]
        && bits_counter_4bits[1] <= bits_counter_4bits[3]
    {
        option[2] = 1;
    } else if bits_counter_4bits[2] <= bits_counter_4bits[1]
        && bits_counter_4bits[2] <= bits_counter_4bits[0]
        && bits_counter_4bits[2] <= bits_counter_4bits[3]
    {
        option[2] = 2;
    }

    Ok(())
}
