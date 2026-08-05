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

/// `scratch_a`/`scratch_b`は呼び出し元が使い回すスクラッチバッファ(`lifting97i.rs`と
/// 同じ理由: 最大サイズで一度確保しておけばレベルが進んでも再割り当てが起きない)。
fn forward_lifting97f(
    x_in: &mut [f32],
    n: usize,
    x_alloc: &mut [f32],
    scratch_a: &mut Vec<f32>,
    scratch_b: &mut Vec<f32>,
) {
    let x_base = F_EXTPAD;
    x_alloc[x_base..x_base + n].copy_from_slice(&x_in[..n]);
    for i in 1..=F_EXTPAD {
        x_alloc[x_base - i] = x_alloc[x_base + i];
        x_alloc[x_base + (n - 1) + i] = x_alloc[x_base + (n - 1) - i];
    }
    let half = n >> 1;
    scratch_a.clear();
    scratch_a.resize(half + 3, 0.0);
    let d = scratch_a;
    scratch_b.clear();
    scratch_b.resize(half + 2, 0.0);
    let r = scratch_b;
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

fn inverse_lifting97f(x: &mut [f32], n: usize, x_alloc: &mut [f32], strict_c_compat: bool) {
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
    // C: `*x++ = (float)(0.788486 * r[0] - 0.0406894 * (r[1] + r[-1]) - ...)`.
    // The unsuffixed constants are `double`, but C's usual arithmetic
    // conversions apply *per operator*, not to the expression as a whole:
    // `r[1] + r[-1]` is a `float + float` (both operands plain array
    // elements), so that addition happens -- and rounds -- in single
    // precision *first*; only the already-float-rounded sum then gets
    // promoted to double when multiplied by the double literal. That's a
    // real (if tiny) accuracy loss versus adding both operands in f64 up
    // front -- confirmed by disassembling a minimal repro (`inversef97f`
    // compiled standalone): gcc emits `addss` (f32 add) then `cvtss2sd`
    // (widen the already-rounded sum) then `mulsd`, for every pairwise term
    // here, rather than widen-then-add. `strict_c_compat` picks which of the
    // two to reproduce: bit-exact match with the C reference decoder, or
    // the strictly more precise f64 accumulation.
    let fp = |a: f32, b: f32| -> f64 {
        if strict_c_compat {
            (a + b) as f64
        } else {
            a as f64 + b as f64
        }
    };
    let f = |v: f32| v as f64;
    for _ in 0..half {
        x[out_i] = (0.788486 * f(x_alloc[r_idx])
            - 0.0406894 * fp(x_alloc[r_idx + 1], x_alloc[r_idx - 1])
            - 0.023849 * fp(x_alloc[d_idx + 1], x_alloc[d_idx - 2])
            + 0.377403 * fp(x_alloc[d_idx], x_alloc[d_idx - 1])) as f32;
        out_i += 1;
        x[out_i] = (0.418092 * fp(x_alloc[r_idx + 1], x_alloc[r_idx])
            - 0.0645389 * fp(x_alloc[r_idx + 2], x_alloc[r_idx - 1])
            - 0.037829 * fp(x_alloc[d_idx + 2], x_alloc[d_idx - 2])
            + 0.110624 * fp(x_alloc[d_idx + 1], x_alloc[d_idx - 1])
            - 0.852699 * f(x_alloc[d_idx])) as f32;
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
    strict_c_compat: bool,
) -> BpeResult<()> {
    if (img_cols % (1 << levels)) != 0 || (img_rows % (1 << levels)) != 0 {
        return Err(BpeError::FileError);
    }
    let mut x_alloc = vec![0f32; img_cols + img_rows + F_EXTPAD + F_EXTPAD + 16];
    let mut buffer = vec![0f32; img_rows];
    let max_half = img_cols.max(img_rows) / 2;
    let mut scratch_a: Vec<f32> = Vec::with_capacity(max_half + 3);
    let mut scratch_b: Vec<f32> = Vec::with_capacity(max_half + 3);
    if !inverse {
        for l in 0..levels {
            let w = img_cols >> l;
            let h = img_rows >> l;
            for y in 0..h {
                forward_lifting97f(
                    &mut rows[y][..w],
                    w,
                    &mut x_alloc,
                    &mut scratch_a,
                    &mut scratch_b,
                );
            }
            for x in 0..w {
                for y in 0..h {
                    buffer[y] = rows[y][x];
                }
                forward_lifting97f(
                    &mut buffer[..h],
                    h,
                    &mut x_alloc,
                    &mut scratch_a,
                    &mut scratch_b,
                );
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
                inverse_lifting97f(&mut buffer[..h], h, &mut x_alloc, strict_c_compat);
                for y in 0..h {
                    rows[y][x] = buffer[y];
                }
            }

            for y in 0..h {
                inverse_lifting97f(&mut rows[y][..w], w, &mut x_alloc, strict_c_compat);
            }

            // l==0's post-state is already covered by the post_idwt seam
            // (dumped by the caller right after this function returns), so
            // only the coarser intermediate levels need a dump here.
            if l != 0 {
                crate::trace::dump_f32_flat(
                    &format!("post_idwt_level{l}_rust.txt"),
                    rows[..img_rows]
                        .iter()
                        .flat_map(|row| row[..img_cols].iter().copied()),
                );
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

    /// Regression test for a real (not just "1 ULP, oh well") bug: an earlier
    /// version of `inverse_lifting97f` converted each array element to f64
    /// *before* adding pairs like `r[1] + r[-1]`, but C's `float + float`
    /// (both plain array elements, no double literal directly involved)
    /// adds -- and rounds -- in f32 *first*, only promoting the sum to f64
    /// when it's multiplied by a double literal coefficient afterward.
    /// Converting-then-adding is strictly more precise than C's actual
    /// per-operator conversion rules, which is a *different* rounding, not
    /// a less-precise one. This input/output pair is real data extracted
    /// from decoding baseline_256 at rate=0.1 (`-t 0 -s 64`, the row-pass
    /// of the coarsest inverse-lifting level, isolated via a temporary
    /// column-pass-only trace point since removed), where 13 of 64 outputs
    /// previously differed from the C reference by ~1 ULP; confirmed via
    /// disassembling a minimal standalone repro of
    /// `inversef97f` (gcc emits `addss` then `cvtss2sd` then `mulsd` for
    /// every such term) that this was the exact mechanism, not an
    /// unavoidable compiler-rounding difference (COMPATIBILITY_REPORT.md
    /// §3.3).
    #[test]
    #[allow(clippy::excessive_precision)] // literals are pinned to the exact C-produced digits
    fn matches_c_reference_on_real_decode_data() {
        let mut x: [f32; 64] = [
            12.21094418,
            80.09323883,
            147.9755249,
            215.8578186,
            283.7401123,
            351.622406,
            419.5046997,
            487.3869934,
            555.2692871,
            623.1515503,
            691.0338745,
            758.9161377,
            826.7984619,
            894.6807251,
            962.5630493,
            1030.445312,
            1106.140015,
            1181.834595,
            1196.649658,
            1211.464722,
            1380.448364,
            390.3999634,
            -225.5737915,
            -124.6475754,
            -48.95292282,
            26.74173355,
            102.4363937,
            178.1310425,
            253.8256989,
            329.5203552,
            405.2150269,
            480.909668,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        // Generated by original/source/inversef97f (gcc -O2) on the exact
        // same 64 inputs -- see the doc comment above for how.
        let expected: [f32; 64] = [
            3.110266924e0,
            2.387233353e1,
            5.663450623e1,
            8.063441467e1,
            1.046345596e2,
            1.286343994e2,
            1.526346130e2,
            1.766343994e2,
            2.006346741e2,
            2.246343994e2,
            2.486347351e2,
            2.726343689e2,
            2.966347961e2,
            3.206343689e2,
            3.446348572e2,
            3.686343384e2,
            3.926349182e2,
            4.166343689e2,
            4.406349487e2,
            4.646343384e2,
            4.886350403e2,
            5.126342773e2,
            5.366350708e2,
            5.606343384e2,
            5.846351318e2,
            6.086343384e2,
            6.326351929e2,
            6.566342773e2,
            6.806352539e2,
            7.041300659e2,
            7.283174438e2,
            7.548922119e2,
            7.821596069e2,
            8.128496094e2,
            8.381608887e2,
            8.448496094e2,
            8.461596069e2,
            8.414464111e2,
            8.503623657e2,
            9.812316895e2,
            1.023285278e3,
            6.767492065e2,
            2.608337402e2,
            -1.213549709e1,
            -1.886750793e2,
            -1.684613647e2,
            -8.711254120e1,
            -5.974857712e1,
            -3.461496353e1,
            -7.852835178e0,
            1.890927124e1,
            4.567132950e1,
            7.243350983e1,
            9.919548798e1,
            1.259577408e2,
            1.527196350e2,
            1.794819794e2,
            2.062438049e2,
            2.330062103e2,
            2.597679749e2,
            2.865304565e2,
            3.181773682e2,
            3.431346436e2,
            3.498247070e2,
        ];
        let mut x_alloc = vec![0f32; 64 + 64 + 4 + 4];
        inverse_lifting97f(&mut x, 64, &mut x_alloc, true);
        assert_eq!(x, expected);
    }

    /// Same real decode-data input, but with `strict_c_compat` off: the
    /// f64-accumulated result must differ from C's per-operator rounding
    /// on at least one of the 13 samples known to diverge (see the test
    /// above), proving the flag actually switches arithmetic paths.
    #[test]
    #[allow(clippy::excessive_precision)] // literals are pinned to the exact C-produced digits
    fn non_strict_mode_diverges_from_c_reference_on_same_input() {
        let mut x: [f32; 64] = [
            12.21094418,
            80.09323883,
            147.9755249,
            215.8578186,
            283.7401123,
            351.622406,
            419.5046997,
            487.3869934,
            555.2692871,
            623.1515503,
            691.0338745,
            758.9161377,
            826.7984619,
            894.6807251,
            962.5630493,
            1030.445312,
            1106.140015,
            1181.834595,
            1196.649658,
            1211.464722,
            1380.448364,
            390.3999634,
            -225.5737915,
            -124.6475754,
            -48.95292282,
            26.74173355,
            102.4363937,
            178.1310425,
            253.8256989,
            329.5203552,
            405.2150269,
            480.909668,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        let c_reference: [f32; 64] = [
            3.110266924e0,
            2.387233353e1,
            5.663450623e1,
            8.063441467e1,
            1.046345596e2,
            1.286343994e2,
            1.526346130e2,
            1.766343994e2,
            2.006346741e2,
            2.246343994e2,
            2.486347351e2,
            2.726343689e2,
            2.966347961e2,
            3.206343689e2,
            3.446348572e2,
            3.686343384e2,
            3.926349182e2,
            4.166343689e2,
            4.406349487e2,
            4.646343384e2,
            4.886350403e2,
            5.126342773e2,
            5.366350708e2,
            5.606343384e2,
            5.846351318e2,
            6.086343384e2,
            6.326351929e2,
            6.566342773e2,
            6.806352539e2,
            7.041300659e2,
            7.283174438e2,
            7.548922119e2,
            7.821596069e2,
            8.128496094e2,
            8.381608887e2,
            8.448496094e2,
            8.461596069e2,
            8.414464111e2,
            8.503623657e2,
            9.812316895e2,
            1.023285278e3,
            6.767492065e2,
            2.608337402e2,
            -1.213549709e1,
            -1.886750793e2,
            -1.684613647e2,
            -8.711254120e1,
            -5.974857712e1,
            -3.461496353e1,
            -7.852835178e0,
            1.890927124e1,
            4.567132950e1,
            7.243350983e1,
            9.919548798e1,
            1.259577408e2,
            1.527196350e2,
            1.794819794e2,
            2.062438049e2,
            2.330062103e2,
            2.597679749e2,
            2.865304565e2,
            3.181773682e2,
            3.431346436e2,
            3.498247070e2,
        ];
        let mut x_alloc = vec![0f32; 64 + 64 + 4 + 4];
        inverse_lifting97f(&mut x, 64, &mut x_alloc, false);
        assert_ne!(x, c_reference);
    }

    #[test]
    fn forward_inverse_restores_samples_within_tolerance() {
        for n in [8usize, 16, 32] {
            let mut data: Vec<f32> = (0..n).map(|i| (i as f32) * 1.5 - 10.0).collect();
            let original = data.clone();
            let mut alloc = vec![0f32; n * 4 + 32];
            let mut scratch_a = Vec::new();
            let mut scratch_b = Vec::new();
            forward_lifting97f(&mut data, n, &mut alloc, &mut scratch_a, &mut scratch_b);
            inverse_lifting97f(&mut data, n, &mut alloc, true);
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
        lifting_f97_2d(&mut image, size, size, 3, false, true).unwrap();
        lifting_f97_2d(&mut image, size, size, 3, true, true).unwrap();
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
        assert!(lifting_f97_2d(&mut image, 12, 12, 3, false, true).is_err());
    }
}
