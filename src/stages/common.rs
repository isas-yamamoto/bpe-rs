//! Shared helpers for stage encode/decode gaggles.
//!
//! Stage layer: Rice option emit/read, rate-stop bookkeeping, and gaggle ranges.

use crate::bitstream::{bits_output, bits_read};
use crate::error::{BpeError, BpeResult};
use crate::pattern::bit_plane_symbol_reset;
use crate::rice::{rice_coding, rice_decoding};
use crate::types::{BitPlaneBits, CodingPara, SymbolDetails, GAGGLE_SIZE};

/// Emit the Rice code-option once per symbol length (sym_len > 1).
pub(super) fn emit_code_option_once(
    coding: &mut CodingPara,
    flag_code_option_output: &mut [bool; 3],
    option: &[u8; 3],
    sym_len: u8,
) -> BpeResult<()> {
    if sym_len > 1 {
        if !flag_code_option_output[(sym_len - 2) as usize] {
            flag_code_option_output[(sym_len - 2) as usize] = true;
            if sym_len == 2 {
                bits_output(coding, option[0] as u32, 1)?;
            } else if sym_len == 3 {
                bits_output(coding, option[1] as u32, 2)?;
            } else if sym_len == 4 {
                bits_output(coding, option[2] as u32, 2)?;
            } else {
                return Err(BpeError::StageCodingError);
            }
        }
    }
    Ok(())
}

/// Count set bits in the low `sym_len` bits of `sym_val`.
pub(super) fn count_ones_in_sym_val(sym_val: u8, sym_len: u8) -> i32 {
    let mut counter: i32 = 0;
    for i in 0..sym_len {
        if (sym_val & (1 << i)) > 0 {
            counter += 1;
        }
    }
    counter
}

/// Rice-encode, optionally emit signs, then reset the symbol.
pub(super) fn rice_then_signs_then_reset(
    coding: &mut CodingPara,
    sym: &mut SymbolDetails,
    option: &[u8; 3],
    emit_signs: bool,
) -> BpeResult<()> {
    rice_coding(coding, sym.sym_mapped_pattern as u32, sym.sym_len, option)?;
    if emit_signs {
        let counter = count_ones_in_sym_val(sym.sym_val, sym.sym_len);
        bits_output(coding, sym.sign as u32, counter)?;
    }
    bit_plane_symbol_reset(sym);
    Ok(())
}

/// True when rate-stop decoding should mark a stop location.
pub(super) fn rate_stop_pending(coding: &CodingPara) -> bool {
    coding.decoding_stop_locations.bit_plane_stop_decoding != -1
        && coding.rate_reached
        && !coding.decoding_stop_locations.location_find
}

/// Record the stop location at (block, x, y).
pub(super) fn mark_stop_at(coding: &mut CodingPara, block: i32, x: i8, y: i8) {
    coding.decoding_stop_locations.block_no_stop_decoding = block;
    coding.decoding_stop_locations.x_location_stop_decoding = x;
    coding.decoding_stop_locations.y_location_stop_decoding = y;
    coding.decoding_stop_locations.location_find = true;
}

/// Read the Rice code-option once when `counter > 1` and the flag is unset.
/// Returns `true` if an option was freshly read (caller should check rate-stop).
/// Caller must only invoke when `counter != 0`.
pub(super) fn read_code_option_once(
    coding: &mut CodingPara,
    flag_code_option_output: &mut [bool; 3],
    code_options: &mut [u8; 3],
    counter: u8,
) -> BpeResult<bool> {
    if counter != 1 {
        if !flag_code_option_output[(counter - 2) as usize] {
            flag_code_option_output[(counter - 2) as usize] = true;
            let temp = if counter == 2 {
                bits_read(coding, 1)?
            } else {
                bits_read(coding, 2)?
            };
            code_options[(counter - 2) as usize] = temp as u8;
            return Ok(true);
        }
    }
    Ok(false)
}

/// TranD-dependent stop coordinates for gaggles2.
pub(super) fn set_trand_stop(coding: &mut CodingPara, block_info: &[BitPlaneBits], bs: usize) {
    coding.decoding_stop_locations.block_no_stop_decoding = bs as i32;
    if (block_info[bs].str_plane_hit_history.tran_d & 0x4) == 0 {
        coding.decoding_stop_locations.x_location_stop_decoding = 0;
        coding.decoding_stop_locations.y_location_stop_decoding = 2;
    } else if (block_info[bs].str_plane_hit_history.tran_d & 0x2) == 0 {
        coding.decoding_stop_locations.x_location_stop_decoding = 2;
        coding.decoding_stop_locations.y_location_stop_decoding = 0;
    } else {
        coding.decoding_stop_locations.x_location_stop_decoding = 2;
        coding.decoding_stop_locations.y_location_stop_decoding = 2;
    }
    coding.decoding_stop_locations.location_find = true;
}

/// TranGi-dependent stop coordinates for gaggles3.
pub(super) fn set_trangi_stop(coding: &mut CodingPara, block_info: &[BitPlaneBits], bs: usize) {
    coding.decoding_stop_locations.block_no_stop_decoding = bs as i32;
    if (block_info[bs].str_plane_hit_history.tran_gi & 0x4) == 0 {
        coding.decoding_stop_locations.x_location_stop_decoding = 0;
        coding.decoding_stop_locations.y_location_stop_decoding = 4;
    } else if (block_info[bs].str_plane_hit_history.tran_gi & 0x2) == 0 {
        coding.decoding_stop_locations.x_location_stop_decoding = 4;
        coding.decoding_stop_locations.y_location_stop_decoding = 0;
    } else {
        coding.decoding_stop_locations.x_location_stop_decoding = 4;
        coding.decoding_stop_locations.y_location_stop_decoding = 4;
    }
    coding.decoding_stop_locations.location_find = true;
}

/// Read optional Rice code-option, then Rice-decode `counter` bits.
/// Returns `(word, stop_pending)` where stop_pending is true if rate-stop
/// fired after the option read (word unused) or after Rice decode.
pub(super) fn read_option_and_rice(
    coding: &mut CodingPara,
    flag_code_option_output: &mut [bool; 3],
    code_options: &mut [u8; 3],
    counter: u8,
) -> BpeResult<(u32, bool)> {
    if read_code_option_once(coding, flag_code_option_output, code_options, counter)? {
        if rate_stop_pending(coding) {
            return Ok((0, true));
        }
    }
    let word = rice_decoding(coding, counter as i16, code_options)?;
    Ok((word, rate_stop_pending(coding)))
}

/// Apply one refinement bit: add or subtract 2^(bit_plane-1) following the sign of `val`.
pub(super) fn apply_refine_delta(val: &mut i32, bit_plane: u8) {
    if *val > 0 {
        *val += 1 << (bit_plane - 1);
    } else {
        *val -= 1 << (bit_plane - 1);
    }
}

/// Yield `(gaggle_index, block_start, blocks_in_gaggle)` for a segment of `s` blocks.
pub(super) fn gaggle_ranges(s: usize) -> impl Iterator<Item = (usize, usize, usize)> {
    let mut total = s / GAGGLE_SIZE;
    if s % GAGGLE_SIZE != 0 {
        total += 1;
    }
    (0..total).map(move |gaggle_index| {
        let start = gaggle_index * GAGGLE_SIZE;
        let len = if start + GAGGLE_SIZE < s {
            GAGGLE_SIZE
        } else {
            s - start
        };
        (gaggle_index, start, len)
    })
}
