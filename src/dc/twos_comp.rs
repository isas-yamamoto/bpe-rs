//! Two-s-complement conversion for DC coefficients - original/source/DC_EnDeCoding.c

use crate::error::{BpeError, BpeResult};

pub fn deconv_twos_comp(complement: u32, leftmost: i16) -> BpeResult<i32> {
    if (leftmost as usize >= 32) || (leftmost == 0) {
        return Err(BpeError::DataError);
    }
    if ((1u32 << (leftmost - 1)) & complement) == 0 {
        Ok(complement as i32)
    } else {
        let mut temp: u32 = 0;
        for _ in 0..leftmost {
            temp <<= 1;
            temp += 1;
        }
        let original = -((((!complement) & temp).wrapping_add(1)) as i32);
        Ok(original)
    }
}

pub fn conv_twos_comp(original: i32, leftmost: i16) -> BpeResult<u32> {
    if leftmost == 1 {
        return Ok(0);
    }
    if (leftmost as usize >= 32) || (leftmost == 0) {
        return Err(BpeError::DataError);
    }
    if original >= 0 {
        Ok(original as u32)
    } else {
        let mut complement: u32 = !((-original) as u32);
        let mut temp: u32 = 0;
        for _ in 0..leftmost {
            temp <<= 1;
            temp += 1;
        }
        complement &= temp;
        complement += 1;
        Ok(complement)
    }
}
