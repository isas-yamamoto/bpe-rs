//! Integer 9/7 lifting — original/source/lifting_97M.c

use crate::error::{BpeError, BpeResult};
use crate::types::ImageI32;

const F_EXTPAD: usize = 4;
const D_EXTPAD: usize = 2;

fn floor_toward_neg_inf_from_temp(temp: f64) -> i32 {
    // Matches C: if temp > 0 then (int)temp else if temp != (int)temp then (int)(temp-1) else (int)temp
    if temp > 0.0 {
        temp as i32
    } else if temp != (temp as i32) as f64 {
        (temp - 1.0) as i32
    } else {
        temp as i32
    }
}

fn forward_lifting97i(x_in: &mut [i32], n: usize, x_alloc: &mut [i32]) {
    let x_base = F_EXTPAD;
    x_alloc[x_base..x_base + n].copy_from_slice(&x_in[..n]);
    for i in 1..=F_EXTPAD {
        x_alloc[x_base - i] = x_alloc[x_base + i];
        x_alloc[x_base + (n - 1) + i] = x_alloc[x_base + (n - 1) - i];
    }

    let half = n >> 1;
    let mut d = vec![0i32; half + 1];
    let mut x_idx = x_base;
    for di in 0..=half {
        let temp = -1.0 / 16.0 * (x_alloc[x_idx - 4] as f64 + x_alloc[x_idx + 2] as f64)
            + 9.0 / 16.0 * (x_alloc[x_idx - 2] as f64 + x_alloc[x_idx] as f64)
            + 0.5;
        d[di] = x_alloc[x_idx - 1] - floor_toward_neg_inf_from_temp(temp);
        x_idx += 2;
    }

    let mut r = vec![0i32; half];
    x_idx = x_base;
    for n_i in 0..half {
        let temp = -0.25 * (d[n_i] as f64 + d[n_i + 1] as f64) + 0.5;
        r[n_i] = x_alloc[x_idx] - floor_toward_neg_inf_from_temp(temp);
        x_idx += 2;
    }

    x_in[..half].copy_from_slice(&r);
    x_in[half..n].copy_from_slice(&d[1..=half]);
}

fn inverse_lifting97i(x: &mut [i32], n: usize, x_alloc: &mut [i32]) {
    let half = n / 2;
    let r_base = D_EXTPAD;
    let d_base = D_EXTPAD + half + D_EXTPAD + D_EXTPAD + D_EXTPAD;
    for i in 0..half {
        x_alloc[r_base + i] = x[i];
        x_alloc[d_base + i] = x[half + i];
    }

    for i in 1..=D_EXTPAD {
        if half <= 1 {
            x_alloc[r_base + (half - 1) + i] = x_alloc[r_base + half - i];
            let v = x_alloc[r_base + half - i];
            x_alloc[r_base - i] = v;
            x_alloc[r_base + i] = v;
        } else {
            x_alloc[r_base - i] = x_alloc[r_base + i];
            x_alloc[r_base + (half - 1) + i] = x_alloc[r_base + half - i];
        }
        x_alloc[d_base - i] = x_alloc[d_base + i - 1];
        x_alloc[d_base + (half - 1) + i] = x_alloc[d_base + half - i - 1];
    }

    let mut x_0 = vec![0i32; half + 3];
    let mut d_idx = d_base;
    let mut r_idx = r_base;
    for i in 0..(half + 3) {
        let rounding = -1.0 / 4.0 * (x_alloc[d_idx - 1] as f64 + x_alloc[d_idx - 2] as f64) + 0.5;
        x_0[i] = x_alloc[r_idx - 1] + floor_toward_neg_inf_from_temp(rounding);
        d_idx += 1;
        r_idx += 1;
    }

    let mut x_1 = vec![0i32; half];
    d_idx = d_base;
    for n_i in 0..half {
        let rounding = -1.0 / 16.0 * (x_0[n_i] as f64 + x_0[n_i + 3] as f64)
            + 9.0 / 16.0 * (x_0[n_i + 1] as f64 + x_0[n_i + 2] as f64)
            + 0.5;
        x_1[n_i] = x_alloc[d_idx] + floor_toward_neg_inf_from_temp(rounding);
        d_idx += 1;
    }

    let x_0s = &x_0[1..];
    for n_i in 0..half {
        x[n_i * 2] = x_0s[n_i];
        x[n_i * 2 + 1] = x_1[n_i];
    }
}

pub fn lifting_m97_2d(
    rows: &mut ImageI32,
    img_rows: usize,
    img_cols: usize,
    levels: i32,
    inverse: bool,
) -> BpeResult<()> {
    if (img_cols % (1 << levels)) != 0 || (img_rows % (1 << levels)) != 0 {
        return Err(BpeError::FileError);
    }
    let mut x_alloc = vec![0i32; img_cols + img_rows + F_EXTPAD + F_EXTPAD];
    let mut buffer = vec![0i32; img_rows];

    if !inverse {
        for l in 0..levels {
            let w = img_cols >> l;
            let h = img_rows >> l;
            for y in 0..h {
                forward_lifting97i(&mut rows[y][..w], w, &mut x_alloc);
            }
            for x in 0..w {
                for y in 0..h {
                    buffer[y] = rows[y][x];
                }
                forward_lifting97i(&mut buffer[..h], h, &mut x_alloc);
                for y in 0..h {
                    rows[y][x] = buffer[y];
                }
            }
        }
    } else {
        for l in (0..levels).rev() {
            let w = img_cols >> l;
            let h = img_rows >> l;
            for x in 0..w {
                for y in 0..h {
                    buffer[y] = rows[y][x];
                }
                inverse_lifting97i(&mut buffer[..h], h, &mut x_alloc);
                for y in 0..h {
                    rows[y][x] = buffer[y];
                }
            }
            for y in 0..h {
                inverse_lifting97i(&mut rows[y][..w], w, &mut x_alloc);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::alloc_image_i32;

    fn sample(n: usize) -> Vec<i32> {
        (0..n as i32).map(|i| (i * 7 % 23) - 11).collect()
    }

    #[test]
    fn forward_inverse_is_lossless_for_several_lengths() {
        for n in [8usize, 16, 32, 64] {
            let mut data = sample(n);
            let original = data.clone();
            let mut alloc = vec![0i32; n * 4 + 16];
            forward_lifting97i(&mut data, n, &mut alloc);
            assert_ne!(data, original, "transform should change the samples");
            inverse_lifting97i(&mut data, n, &mut alloc);
            assert_eq!(data, original, "length {} must round-trip exactly", n);
        }
    }

    #[test]
    fn constant_signal_keeps_detail_bands_at_zero() {
        let n = 16;
        let mut data = vec![100i32; n];
        let mut alloc = vec![0i32; n * 4 + 16];
        forward_lifting97i(&mut data, n, &mut alloc);
        assert!(
            data[n / 2..].iter().all(|&v| v == 0),
            "a flat signal must not produce detail coefficients: {:?}",
            data
        );
    }

    #[test]
    fn two_dimensional_transform_is_lossless() {
        let size = 32;
        let mut image = alloc_image_i32(size, size);
        for y in 0..size {
            for x in 0..size {
                image[y][x] = ((x * 3 + y * 5) % 251) as i32 - 125;
            }
        }
        let original = image.clone();
        lifting_m97_2d(&mut image, size, size, 3, false).unwrap();
        lifting_m97_2d(&mut image, size, size, 3, true).unwrap();
        assert_eq!(image, original);
    }

    #[test]
    fn size_not_divisible_by_levels_is_rejected() {
        let mut image = alloc_image_i32(12, 12);
        assert!(lifting_m97_2d(&mut image, 12, 12, 3, false).is_err());
    }
}
