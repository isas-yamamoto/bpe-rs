//! Pattern mapping/demapping and symbol reset - original/source/PatternCoding.c

use crate::error::{BpeError, BpeResult};
use crate::types::SymbolDetails;

const BIT2_PATTERN: [u8; 4] = [0, 2, 1, 3];
const BIT3_PATTERN: [u8; 8] = [1, 4, 0, 5, 2, 6, 3, 7];
const BIT3_PATTERN_TRAN_D: [u8; 8] = [0, 3, 0, 4, 1, 5, 2, 6];
const BIT4_PATTERN_TYPE_CI: [u8; 16] = [10, 1, 3, 6, 2, 5, 9, 12, 0, 8, 7, 13, 4, 14, 11, 15];
const BIT4_PATTERN_TYPE_HIJ_TRAN_HI: [u8; 16] =
    [0, 1, 3, 6, 2, 5, 9, 11, 0, 8, 7, 12, 4, 13, 10, 14];

const ENUM_TRAN_D: u8 = crate::types::ENUM_TRAN_D;
const ENUM_TYPE_CI: u8 = crate::types::ENUM_TYPE_CI;
const ENUM_TRAN_HI: u8 = crate::types::ENUM_TRAN_HI;
const ENUM_TYPE_HIJ: u8 = crate::types::ENUM_TYPE_HIJ;

pub fn pattern_mapping(sym: &mut SymbolDetails) -> BpeResult<()> {
    match sym.sym_len {
        0 => Ok(()),
        1 => {
            sym.sym_mapped_pattern = sym.sym_val;
            Ok(())
        }
        2 => {
            sym.sym_mapped_pattern = BIT2_PATTERN[sym.sym_val as usize];
            Ok(())
        }
        3 => {
            if sym.type_ == ENUM_TRAN_D {
                sym.sym_mapped_pattern = BIT3_PATTERN_TRAN_D[sym.sym_val as usize];
            } else {
                sym.sym_mapped_pattern = BIT3_PATTERN[sym.sym_val as usize];
            }
            Ok(())
        }
        4 => {
            if sym.type_ == ENUM_TYPE_CI {
                sym.sym_mapped_pattern = BIT4_PATTERN_TYPE_CI[sym.sym_val as usize];
            } else if sym.type_ == ENUM_TRAN_HI || sym.type_ == ENUM_TYPE_HIJ {
                sym.sym_mapped_pattern = BIT4_PATTERN_TYPE_HIJ_TRAN_HI[sym.sym_val as usize];
            }
            Ok(())
        }
        _ => Err(BpeError::PatternCodingError),
    }
}

pub fn de_mapping_pattern(sym: &mut SymbolDetails) -> BpeResult<()> {
    match sym.sym_len {
        1 => {
            sym.sym_val = sym.sym_mapped_pattern;
            Ok(())
        }
        2 => {
            for i in 0..4u8 {
                if sym.sym_mapped_pattern == BIT2_PATTERN[i as usize] {
                    sym.sym_val = i;
                    break;
                }
            }
            Ok(())
        }
        3 => {
            if sym.type_ == ENUM_TRAN_D {
                for i in 1..8u8 {
                    if sym.sym_mapped_pattern == BIT3_PATTERN_TRAN_D[i as usize] {
                        sym.sym_val = i;
                        break;
                    }
                }
            } else {
                for i in 0..8u8 {
                    if sym.sym_mapped_pattern == BIT3_PATTERN[i as usize] {
                        sym.sym_val = i;
                        break;
                    }
                }
            }
            Ok(())
        }
        4 => {
            if sym.type_ == ENUM_TYPE_CI {
                for i in 0..16u8 {
                    if sym.sym_mapped_pattern == BIT4_PATTERN_TYPE_CI[i as usize] {
                        sym.sym_val = i;
                        break;
                    }
                }
            } else if sym.type_ == ENUM_TRAN_HI || sym.type_ == ENUM_TYPE_HIJ {
                for i in 1..16u8 {
                    if sym.sym_mapped_pattern == BIT4_PATTERN_TYPE_HIJ_TRAN_HI[i as usize] {
                        sym.sym_val = i;
                        break;
                    }
                }
            }
            Ok(())
        }
        _ => Err(BpeError::PatternCodingError),
    }
}

pub fn bit_plane_symbol_reset(sym: &mut SymbolDetails) {
    sym.sign = 0;
    sym.sym_len = 0;
    sym.sym_mapped_pattern = 0;
    sym.sym_val = 0;
    sym.type_ = 0;
}
