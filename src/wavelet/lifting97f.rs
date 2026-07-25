//! Float 9/7 lifting — original/source/lifting_97f.c

use crate::error::{BpeError, BpeResult};
use crate::types::ImageF32;

const F_EXTPAD: usize = 4;
const D_EXTPAD: usize = 2;

const LOW_PASS_FILTER: [f32; 9] = [
    0.037828455507,
    -0.023849465020,
    -0.110624404418,
    0.377402855613,
    0.852698679009,
    0.377402855613,
    -0.110624404418,
    -0.023849465020,
    0.037828455507,
];
const HIGH_PASS_FILTER: [f32; 7] = [
    -0.064538882629,
    0.040689417609,
    0.418092273222,
    -0.788485616406,
    0.418092273222,
    0.040689417609,
    -0.064538882629,
];

fn forward_lifting97f(x_in: &mut [f32], n: usize, x_alloc: &mut [f32]) {
    let x_base = F_EXTPAD;
    x_alloc[x_base..x_base + n].copy_from_slice(&x_in[..n]);
    for i in 1..=F_EXTPAD {
        x_alloc[x_base - i] = x_alloc[x_base + i];
        x_alloc[x_base + (n - 1) + i] = x_alloc[x_base + (n - 1) - i];
    }
    let half = n >> 1;
    let mut d = vec![0f32; half + 3];
    let mut r = vec![0f32; half + 2];
    let lpf = &LOW_PASS_FILTER[4..]; // LPF[0] at center
                                     // C: LPF = LowPassFilter + 4, so LPF[0]=LowPassFilter[4], LPF[1]=[5]... but uses LPF[0]..[4]
                                     // Actually LowPassFilter has 9 elements, +4 points to index 4 (center 0.852...)
                                     // LPF[0]=0.852..., LPF[1]=0.377..., LPF[2]=-0.110..., LPF[3]=-0.023..., LPF[4]=0.037...
                                     // Wait C: float const *LPF = LowPassFilter + 4; uses LPF[0]..LPF[4]
                                     // LowPassFilter[4]=0.852698679009, [5]=0.377..., [6]=-0.110..., [7]=-0.023..., [8]=0.037...
                                     // But also LPF[-something]? No only non-negative indices.
                                     // Actually looking at the formula: LPF[0]*x[2n] + LPF[1]*(x[2n-1]+x[2n+1]) + ...
                                     // With LPF = LowPassFilter+4: LPF[0]=filter[4], LPF[1]=filter[5], etc.
                                     // HPF = HighPassFilter + 3: HPF[0]=filter[3]=-0.788..., HPF[1]=0.418..., etc.

    let lpf0 = LOW_PASS_FILTER[4];
    let lpf1 = LOW_PASS_FILTER[5];
    let lpf2 = LOW_PASS_FILTER[6];
    let lpf3 = LOW_PASS_FILTER[7];
    let lpf4 = LOW_PASS_FILTER[8];
    let hpf0 = HIGH_PASS_FILTER[3];
    let hpf1 = HIGH_PASS_FILTER[4];
    let hpf2 = HIGH_PASS_FILTER[5];
    let hpf3 = HIGH_PASS_FILTER[6];

    for n_i in 0..half {
        let x = |k: isize| x_alloc[(x_base as isize + k) as usize];
        let tn = (2 * n_i) as isize;
        d[n_i] = lpf0 * x(tn)
            + lpf1 * (x(tn - 1) + x(tn + 1))
            + lpf2 * (x(tn - 2) + x(tn + 2))
            + lpf3 * (x(tn - 3) + x(tn + 3))
            + lpf4 * (x(tn - 4) + x(tn + 4));
        r[n_i] = hpf0 * x(tn + 1)
            + hpf1 * (x(tn) + x(tn + 2))
            + hpf2 * (x(tn - 1) + x(tn + 3))
            + hpf3 * (x(tn - 2) + x(tn + 4));
    }
    x_in[..half].copy_from_slice(&d[..half]);
    x_in[half..n].copy_from_slice(&r[..half]);
    let _ = (lpf,);
}

fn inverse_lifting97f(x: &mut [f32], n: usize, x_alloc: &mut [f32]) {
    let half = n / 2;
    let r_base = D_EXTPAD;
    let d_base = D_EXTPAD + half + D_EXTPAD + D_EXTPAD;
    for i in 0..half {
        x_alloc[r_base + i] = x[i];
        x_alloc[d_base + i] = x[half + i];
    }
    for i in 1..=D_EXTPAD {
        x_alloc[r_base - i] = x_alloc[r_base + i];
        x_alloc[r_base + (half - 1) + i] = x_alloc[r_base + half - i];
        x_alloc[d_base - i] = x_alloc[d_base + i - 1];
        x_alloc[d_base + (half - 1) + i] = x_alloc[d_base + (half - 1) - i];
    }
    let mut out_i = 0usize;
    let mut d_idx = d_base;
    let mut r_idx = r_base;
    for _ in 0..half {
        x[out_i] = 0.788486 * x_alloc[r_idx]
            - 0.0406894 * (x_alloc[r_idx + 1] + x_alloc[r_idx - 1])
            - 0.023849 * (x_alloc[d_idx + 1] + x_alloc[d_idx - 2])
            + 0.377403 * (x_alloc[d_idx] + x_alloc[d_idx - 1]);
        out_i += 1;
        x[out_i] = 0.418092 * (x_alloc[r_idx + 1] + x_alloc[r_idx])
            - 0.0645389 * (x_alloc[r_idx + 2] + x_alloc[r_idx - 1])
            - 0.037829 * (x_alloc[d_idx + 2] + x_alloc[d_idx - 2])
            + 0.110624 * (x_alloc[d_idx + 1] + x_alloc[d_idx - 1])
            - 0.852699 * x_alloc[d_idx];
        out_i += 1;
        d_idx += 1;
        r_idx += 1;
    }
}

pub fn lifting_f97_2d(
    rows: &mut ImageF32,
    img_rows: usize,
    img_cols: usize,
    levels: i32,
    inverse: bool,
) -> BpeResult<()> {
    if (img_cols % (1 << levels)) != 0 || (img_rows % (1 << levels)) != 0 {
        return Err(BpeError::FileError);
    }
    let mut x_alloc = vec![0f32; img_cols + img_rows + F_EXTPAD + F_EXTPAD + 16];
    let mut buffer = vec![0f32; img_rows];
    if !inverse {
        for l in 0..levels {
            let w = img_cols >> l;
            let h = img_rows >> l;
            for y in 0..h {
                forward_lifting97f(&mut rows[y][..w], w, &mut x_alloc);
            }
            for x in 0..w {
                for y in 0..h {
                    buffer[y] = rows[y][x];
                }
                forward_lifting97f(&mut buffer[..h], h, &mut x_alloc);
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
                inverse_lifting97f(&mut buffer[..h], h, &mut x_alloc);
                for y in 0..h {
                    rows[y][x] = buffer[y];
                }
            }
            for y in 0..h {
                inverse_lifting97f(&mut rows[y][..w], w, &mut x_alloc);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::alloc_image_f32;

    const TOLERANCE: f32 = 1e-2;

    #[test]
    fn forward_inverse_restores_samples_within_tolerance() {
        for n in [8usize, 16, 32] {
            let mut data: Vec<f32> = (0..n).map(|i| (i as f32) * 1.5 - 10.0).collect();
            let original = data.clone();
            let mut alloc = vec![0f32; n * 4 + 32];
            forward_lifting97f(&mut data, n, &mut alloc);
            inverse_lifting97f(&mut data, n, &mut alloc);
            for (got, want) in data.iter().zip(original.iter()) {
                assert!(
                    (got - want).abs() < TOLERANCE,
                    "length {}: got {} want {}",
                    n,
                    got,
                    want
                );
            }
        }
    }

    #[test]
    fn two_dimensional_transform_restores_image_within_tolerance() {
        let size = 16;
        let mut image = alloc_image_f32(size, size);
        for y in 0..size {
            for x in 0..size {
                image[y][x] = ((x * 5 + y * 3) % 97) as f32;
            }
        }
        let original = image.clone();
        lifting_f97_2d(&mut image, size, size, 3, false).unwrap();
        lifting_f97_2d(&mut image, size, size, 3, true).unwrap();
        for y in 0..size {
            for x in 0..size {
                assert!(
                    (image[y][x] - original[y][x]).abs() < TOLERANCE,
                    "pixel ({}, {}): got {} want {}",
                    x,
                    y,
                    image[y][x],
                    original[y][x]
                );
            }
        }
    }

    #[test]
    fn size_not_divisible_by_levels_is_rejected() {
        let mut image = alloc_image_f32(12, 12);
        assert!(lifting_f97_2d(&mut image, 12, 12, 3, false).is_err());
    }
}
