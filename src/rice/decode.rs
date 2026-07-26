//! Fixed-table Rice decode for AC stage symbols.

use crate::bitstream::bits_read;
use crate::error::{BpeError, BpeResult};
use crate::types::CodingPara;

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
