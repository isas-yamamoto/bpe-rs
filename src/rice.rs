//! Rice coding — original/source/ricecoding.c
//!
//! `option` corresponds to the C `UCHAR8 *Option` / `splitOption` array indexed
//! as `option[0]` (2-bit codewords), `option[1]` (3-bit codewords) and
//! `option[2]` (4-bit codewords).

use crate::bitstream::{bits_output, bits_read};
use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

pub fn rice_coding(
    coding: &mut CodingPara,
    input_val: u32,
    bit_length: u8,
    option: &[u8; 3],
) -> BpeResult<()> {
    match bit_length {
        0 => Ok(()),
        1 => bits_output(coding, input_val, 1),
        2 => {
            if option[0] == 1 {
                bits_output(coding, input_val, 2)
            } else if option[0] == 0 {
                if input_val <= 2 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else {
                    bits_output(coding, 0, 3)
                }
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        3 => {
            if option[1] == 0 {
                if input_val <= 2 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else if input_val <= 5 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val - 3, 2)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val, 3)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 1 {
                if input_val <= 1 {
                    bits_output(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_output(coding, input_val, 3)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 2)?;
                    match input_val {
                        4 => bits_output(coding, 2, 2),
                        5 => bits_output(coding, 3, 2),
                        6 => bits_output(coding, 0, 2),
                        7 => bits_output(coding, 1, 2),
                        _ => unreachable!(),
                    }
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[1] == 3 {
                bits_output(coding, input_val, 3)
            } else {
                Ok(())
            }
        }
        4 => {
            if option[2] == 0 {
                if input_val <= 3 {
                    bits_output(coding, 0, input_val as i32)?;
                    bits_output(coding, 1, 1)
                } else if input_val <= 7 {
                    bits_output(coding, 0, 5)?;
                    bits_output(coding, input_val - 4, 2)
                } else if input_val <= 15 {
                    bits_output(coding, 0, 4)?;
                    bits_output(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 1 {
                if input_val <= 1 {
                    bits_output(coding, input_val + 2, 2)
                } else if input_val <= 3 {
                    bits_output(coding, input_val, 3)
                } else if input_val <= 5 {
                    bits_output(coding, 0, 2)?;
                    bits_output(coding, input_val - 2, 2)
                } else if input_val <= 11 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val - 6, 3)
                } else if input_val <= 15 {
                    bits_output(coding, 0, 3)?;
                    bits_output(coding, input_val, 4)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 2 {
                if input_val <= 3 {
                    bits_output(coding, input_val + 4, 3)
                } else if input_val <= 7 {
                    bits_output(coding, input_val, 4)
                } else if input_val <= 11 {
                    bits_output(coding, 0, 2)?;
                    bits_output(coding, input_val - 4, 3)
                } else if input_val <= 15 {
                    bits_output(coding, input_val - 12, 5)
                } else {
                    Err(BpeError::RiceCodingError)
                }
            } else if option[2] == 3 {
                bits_output(coding, input_val, 4)
            } else {
                Err(BpeError::RiceCodingError)
            }
        }
        _ => Err(BpeError::RiceCodingError),
    }
}

/// Faithfully mirrors the C control flow: after every `BitsRead` call, if
/// `coding.rate_reached` became true the decoded value is forced to 0 and
/// decoding stops early (matching the many `*decoded = 0; return;` sites).
pub fn rice_decoding(
    coding: &mut CodingPara,
    bit_length: i16,
    split_option: &[u8; 3],
) -> BpeResult<u32> {
    match bit_length {
        0 => Ok(0),
        1 => {
            let word = bits_read(coding, 1)?;
            Ok(word)
        }
        2 => match split_option[0] {
            1 => {
                let word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            0 => {
                let mut i: u32 = 0;
                while i < 3 {
                    let word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                Ok(i)
            }
            _ => Err(BpeError::RiceCodingError),
        },
        3 => match split_option[1] {
            3 => {
                let word = bits_read(coding, 3)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            1 => {
                let mut word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if (word & 0x2) > 0 {
                    if (word & 0x1) == 0 {
                        Ok(0)
                    } else {
                        Ok(1)
                    }
                } else if (word & 0x1) > 0 {
                    word = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word > 0 {
                        Ok(3)
                    } else {
                        Ok(2)
                    }
                } else {
                    word = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match word {
                        0x2 => Ok(4),
                        0x3 => Ok(5),
                        0x0 => Ok(6),
                        0x1 => Ok(7),
                        _ => unreachable!(),
                    }
                }
            }
            0 => {
                let mut word_readin: u32 = 0;
                let mut i: u32 = 0;
                while i < 3 {
                    word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                if word_readin == 1 {
                    Ok(i)
                } else {
                    let word = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word != 3 {
                        Ok(word + 3)
                    } else {
                        let word2 = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if word2 == 0 {
                            Ok(6)
                        } else {
                            Ok(7)
                        }
                    }
                }
            }
            _ => Err(BpeError::RiceCodingError),
        },
        4 => match split_option[2] {
            3 => {
                let word = bits_read(coding, 4)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                Ok(word)
            }
            2 => {
                let word = bits_read(coding, 3)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if (word & 0x4) > 0 {
                    match word & 0x3 {
                        0x0 => Ok(0),
                        0x1 => Ok(1),
                        0x2 => Ok(2),
                        0x3 => Ok(3),
                        _ => unreachable!(),
                    }
                } else if (word & 0x2) > 0 {
                    if (word & 0x1) == 0 {
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w == 0 {
                            Ok(4)
                        } else {
                            Ok(5)
                        }
                    } else {
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w == 0 {
                            Ok(6)
                        } else {
                            Ok(7)
                        }
                    }
                } else if (word & 0x1) == 1 {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match w {
                        0x0 => Ok(8),
                        0x1 => Ok(9),
                        0x2 => Ok(10),
                        0x3 => Ok(11),
                        _ => unreachable!(),
                    }
                } else {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    match w {
                        0x0 => Ok(12),
                        0x1 => Ok(13),
                        0x2 => Ok(14),
                        0x3 => Ok(15),
                        _ => unreachable!(),
                    }
                }
            }
            1 => {
                let word = bits_read(coding, 2)?;
                if coding.rate_reached {
                    return Ok(0);
                }
                if word >= 2 {
                    if word == 2 {
                        Ok(0)
                    } else {
                        Ok(1)
                    }
                } else if (word & 1) == 1 {
                    let w = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if w == 0 {
                        Ok(2)
                    } else {
                        Ok(3)
                    }
                } else {
                    let w = bits_read(coding, 2)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if (w & 0x2) > 0 {
                        if (w & 0x1) == 0 {
                            Ok(4)
                        } else {
                            Ok(5)
                        }
                    } else if (w & 0x1) == 0 {
                        let w2 = bits_read(coding, 2)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        match w2 {
                            0x0 => Ok(6),
                            0x1 => Ok(7),
                            0x2 => Ok(8),
                            0x3 => Ok(9),
                            _ => unreachable!(),
                        }
                    } else {
                        let w2 = bits_read(coding, 2)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        if w2 == 0 {
                            Ok(10)
                        } else if w2 == 1 {
                            Ok(11)
                        } else if w2 == 2 {
                            let w3 = bits_read(coding, 1)?;
                            if coding.rate_reached {
                                return Ok(0);
                            }
                            if w3 == 0 {
                                Ok(12)
                            } else {
                                Ok(13)
                            }
                        } else {
                            let w3 = bits_read(coding, 1)?;
                            if coding.rate_reached {
                                return Ok(0);
                            }
                            if w3 == 0 {
                                Ok(14)
                            } else {
                                Ok(15)
                            }
                        }
                    }
                }
            }
            0 => {
                let mut i: u32 = 0;
                while i < 4 {
                    let word_readin = bits_read(coding, 1)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if word_readin == 1 {
                        break;
                    }
                    i += 1;
                }
                if i != 4 {
                    Ok(i)
                } else {
                    let word = bits_read(coding, 3)?;
                    if coding.rate_reached {
                        return Ok(0);
                    }
                    if (word & 0x4) == 0 {
                        match word {
                            0x0 => Ok(4),
                            0x1 => Ok(5),
                            0x2 => Ok(6),
                            0x3 => Ok(7),
                            _ => unreachable!(),
                        }
                    } else {
                        let mut decoded = word;
                        decoded <<= 1;
                        let w = bits_read(coding, 1)?;
                        if coding.rate_reached {
                            return Ok(0);
                        }
                        decoded += w;
                        Ok(decoded)
                    }
                }
            }
            _ => Err(BpeError::RiceCodingError),
        },
        _ => Err(BpeError::RiceCodingError),
    }
}

/// Sentinel k value meaning "emit uncoded words for this gaggle".
pub(crate) const UNCODED_FLAG: i32 = 0xFF;

// Heuristic thresholds matching the original C DC/AC gaggle k selection.
const HEUR_UNCODED_DELTA_MUL: i64 = 64;
const HEUR_UNCODED_J_MUL: i64 = 23;
const HEUR_K0_J_MUL: i64 = 207;
const HEUR_K0_DELTA_MUL: i64 = 128;
const HEUR_LARGE_K_SHIFT: i64 = 5;
const HEUR_LARGE_K_DELTA_MUL: i64 = 128;
const HEUR_LARGE_K_J_MUL: i64 = 49;
const HEUR_SCAN_SHIFT_BASE: i64 = 7;
const HEUR_SCAN_DELTA_MUL: i64 = 128;
const HEUR_SCAN_J_MUL: i64 = 49;

/// Choose Rice parameter `k` for one gaggle of mapped magnitudes.
///
/// `mapped[i]` is the mapped DC or AC value for block `start_index + i` style
/// absolute indices via the slice covering `[start_index, start_index+gaggles)`.
/// `opt_select` mirrors `header.part3.opt_dc_select` (exhaustive search vs heuristic).
pub(crate) fn select_rice_k(
    mapped: &[u32],
    start_index: usize,
    gaggles: usize,
    n: u8,
    max_k: i32,
    opt_select: bool,
) -> i32 {
    if opt_select {
        let mut min_k = UNCODED_FLAG;
        let mut min_bits: u32 = 0xFFFF;
        for k in 0..=max_k {
            let mut total_bits: u32 = if start_index == 0 { n as u32 } else { 0 };
            for i in start_index.max(1)..(start_index + gaggles) {
                total_bits += ((mapped[i] >> k) + 1) + k as u32;
            }
            if (total_bits < min_bits) && (total_bits < n as u32 * gaggles as u32) {
                min_bits = total_bits;
                min_k = k;
            }
        }
        min_k
    } else {
        let mut delta: i64 = 0;
        let mut j = gaggles as i64;
        if start_index == 0 {
            j = gaggles as i64 - 1;
        }
        for i in start_index..(start_index + gaggles) {
            delta += mapped[i] as i64;
        }
        if HEUR_UNCODED_DELTA_MUL * delta >= HEUR_UNCODED_J_MUL * j * (1i64 << n) {
            UNCODED_FLAG
        } else if HEUR_K0_J_MUL * j > HEUR_K0_DELTA_MUL * delta {
            0
        } else if j * (1i64 << (n as i64 + HEUR_LARGE_K_SHIFT))
            <= HEUR_LARGE_K_DELTA_MUL * delta + HEUR_LARGE_K_J_MUL * j
        {
            n as i32 - 2
        } else {
            let mut min_k = 0;
            while j * (1i64 << (min_k as i64 + HEUR_SCAN_SHIFT_BASE))
                <= HEUR_SCAN_DELTA_MUL * delta + HEUR_SCAN_J_MUL * j
            {
                min_k += 1;
            }
            min_k - 1
        }
    }
}
