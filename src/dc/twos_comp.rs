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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_over_full_range() {
        for leftmost in 2i16..=16 {
            let bound = 1i32 << (leftmost - 1);
            for original in [-bound, -bound + 1, -1, 0, 1, bound - 2, bound - 1] {
                let encoded = conv_twos_comp(original, leftmost).unwrap();
                let decoded = deconv_twos_comp(encoded, leftmost).unwrap();
                assert_eq!(decoded, original, "leftmost={}", leftmost);
            }
        }
    }

    #[test]
    fn positive_values_are_unchanged() {
        assert_eq!(conv_twos_comp(5, 8).unwrap(), 5);
        assert_eq!(deconv_twos_comp(5, 8).unwrap(), 5);
    }

    #[test]
    fn single_bit_width_collapses_to_zero() {
        assert_eq!(conv_twos_comp(-1, 1).unwrap(), 0);
        assert_eq!(conv_twos_comp(7, 1).unwrap(), 0);
    }

    #[test]
    fn invalid_width_is_rejected() {
        assert!(conv_twos_comp(1, 0).is_err());
        assert!(conv_twos_comp(1, 32).is_err());
        assert!(deconv_twos_comp(1, 0).is_err());
        assert!(deconv_twos_comp(1, 32).is_err());
    }
}
