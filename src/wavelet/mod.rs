//! Wavelet transform — original/source/waveletbpe.c

pub mod coeff_group;
pub mod lifting97f;
pub mod lifting97i;

pub use lifting97f::lifting_f97_2d;
pub use lifting97i::lifting_m97_2d;

use crate::error::{BpeError, BpeResult};
use crate::types::{
    alloc_image_f32, CodingPara, HeaderPart4, ImageF32, ImageI32, FLOAT_WAVELET, INTEGER_WAVELET,
};

fn band_scales(part4: &HeaderPart4) -> [i32; 10] {
    if part4.custom_wt_flag {
        [
            1 << part4.custom_wt_hh1,
            1 << part4.custom_wt_hl1,
            1 << part4.custom_wt_lh1,
            1 << part4.custom_wt_hh2,
            1 << part4.custom_wt_hl2,
            1 << part4.custom_wt_lh2,
            1 << part4.custom_wt_hh3,
            1 << part4.custom_wt_hl3,
            1 << part4.custom_wt_lh3,
            1 << part4.custom_wt_ll3,
        ]
    } else {
        [1, 2, 2, 2, 4, 4, 4, 8, 8, 8]
    }
}

/// Band regions in C order: HH1, HL1, LH1, HH2, HL2, LH2, HH3, HL3, LH3, LL3.
/// Returns (row_start, row_end, col_start, col_end) per scale index.
fn band_regions(rows: usize, cols: usize) -> [(usize, usize, usize, usize); 10] {
    [
        (rows >> 1, rows, cols >> 1, cols),           // HH1
        (0, rows >> 1, cols >> 1, cols),              // HL1
        (rows >> 1, rows, 0, cols >> 1),              // LH1
        (rows >> 2, rows >> 1, cols >> 2, cols >> 1), // HH2
        (0, rows >> 2, cols >> 2, cols >> 1),         // HL2
        (rows >> 2, rows >> 1, 0, cols >> 2),         // LH2
        (rows >> 3, rows >> 2, cols >> 3, cols >> 2), // HH3
        (0, rows >> 3, cols >> 3, cols >> 2),         // HL3
        (rows >> 3, rows >> 2, 0, cols >> 3),         // LH3
        (0, rows >> 3, 0, cols >> 3),                 // LL3
    ]
}

pub fn coefficients_scaling(
    transformed: &mut ImageI32,
    rows: usize,
    cols: usize,
    part4: &HeaderPart4,
) {
    let scales = band_scales(part4);
    for (idx, &(r0, r1, c0, c1)) in band_regions(rows, cols).iter().enumerate() {
        for i in r0..r1 {
            for j in c0..c1 {
                transformed[i][j] *= scales[idx];
            }
        }
    }
}

/// Integer divide toward zero, matching C.
pub fn coefficients_rescaling(
    transformed: &mut ImageI32,
    rows: usize,
    cols: usize,
    part4: &HeaderPart4,
) {
    let scales = band_scales(part4);
    for (idx, &(r0, r1, c0, c1)) in band_regions(rows, cols).iter().enumerate() {
        for i in r0..r1 {
            for j in c0..c1 {
                transformed[i][j] /= scales[idx];
            }
        }
    }
}

#[inline]
fn round_away_from_zero(v: f32) -> i32 {
    // C: `(int)(v + 0.5)` — the unsuffixed 0.5 literal is `double`, so the add
    // happens in f64 before truncation. Matching that (instead of adding an
    // f32 0.5) matters: values whose fractional part sits within one f32 ULP
    // of 0.5 round differently in f32 arithmetic than in f64 arithmetic.
    if v >= 0.0 {
        (v as f64 + 0.5) as i32
    } else {
        (v as f64 - 0.5) as i32
    }
}

pub fn dwt_forward(coding: &CodingPara, imgin: &ImageI32, img_wav: &mut ImageI32) -> BpeResult<()> {
    let pad_rows = (coding.image_rows + coding.header.part1.pad_rows_3bits as u32) as usize;
    let pad_cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;

    match coding.header.part4.dwt_type {
        FLOAT_WAVELET => {
            let mut f97 = alloc_image_f32(pad_rows, pad_cols);
            for i in 0..pad_rows {
                for j in 0..pad_cols {
                    f97[i][j] = imgin[i][j] as f32;
                }
            }
            lifting_f97_2d(&mut f97, pad_rows, pad_cols, 3, false)?;
            for i in 0..pad_rows {
                for j in 0..pad_cols {
                    img_wav[i][j] = round_away_from_zero(f97[i][j]);
                }
            }
            // C also regroups f97 but does not copy it back into img_wav.
            coeff_regroup_f97(&mut f97, pad_rows, pad_cols);
        }
        INTEGER_WAVELET => {
            for i in 0..pad_rows {
                for j in 0..pad_cols {
                    img_wav[i][j] = imgin[i][j];
                }
            }
            lifting_m97_2d(img_wav, pad_rows, pad_cols, 3, false)?;
            coefficients_scaling(img_wav, pad_rows, pad_cols, &coding.header.part4);
        }
        _ => return Err(BpeError::WaveletInvalid),
    }
    coeff_regroup(img_wav, pad_rows, pad_cols);
    crate::trace::dump_i32_flat(
        "dwt_forward_rust.txt",
        img_wav[..pad_rows]
            .iter()
            .flat_map(|row| row[..pad_cols].iter().copied()),
    );
    Ok(())
}

/// Integer path / float-via-int path.
pub fn dwt_reverse(block: &mut ImageI32, coding: &CodingPara) -> BpeResult<()> {
    let rows = coding.image_rows as usize;
    let cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;

    if coding.header.part4.dwt_type == FLOAT_WAVELET {
        let mut temp_f = alloc_image_f32(rows, cols);
        for k in 0..rows {
            for p in 0..cols {
                temp_f[k][p] = block[k][p] as f32;
            }
        }
        // C passes ImageWidth without pad for this call.
        lifting_f97_2d(&mut temp_f, rows, coding.image_width as usize, 3, true)?;
        for k in 0..rows {
            for p in 0..cols {
                block[k][p] = temp_f[k][p] as i32;
            }
        }
    } else if coding.header.part4.dwt_type == INTEGER_WAVELET {
        coefficients_rescaling(block, rows, cols, &coding.header.part4);
        lifting_m97_2d(block, rows, cols, 3, true)?;
    }
    Ok(())
}

pub fn dwt_reverse_floating(block: &mut ImageF32, coding: &CodingPara) -> BpeResult<()> {
    let rows = coding.image_rows as usize;
    let cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;
    let mut temp_f = alloc_image_f32(rows, cols);
    for k in 0..rows {
        for p in 0..coding.image_width as usize {
            temp_f[k][p] = block[k][p];
        }
    }
    lifting_f97_2d(&mut temp_f, rows, cols, 3, true)?;
    for k in 0..rows {
        for p in 0..coding.image_width as usize {
            let v = temp_f[k][p];
            // C: `(float)(v + 0.5)` — add happens in f64 (unsuffixed literal),
            // then narrows to f32. See round_away_from_zero above for why
            // that differs from a straight f32 add near the 0.5 boundary.
            block[k][p] = if v >= 0.0 {
                (v as f64 + 0.5) as f32
            } else {
                (v as f64 - 0.5) as f32
            };
        }
    }
    Ok(())
}

pub use coeff_group::{coeff_degroup, coeff_degroup_floating, coeff_regroup, coeff_regroup_f97};
