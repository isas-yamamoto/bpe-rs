//! Coefficient regroup/degroup — original/source/CoeffGroup.c

use crate::types::{ImageF32, ImageI32};

fn alloc_temp<T: Copy + Default>(rows: usize, cols: usize) -> Vec<Vec<T>> {
    vec![vec![T::default(); cols]; rows]
}

/// Generic CoeffDegroup body (temp buffer then copy-back).
fn degroup_generic<T: Copy + Default>(img_wav: &mut Vec<Vec<T>>, rows: usize, cols: usize) {
    let mut temp = alloc_temp::<T>(rows, cols);
    // HH1
    let mut i = rows >> 1;
    while i < rows {
        let mut j = cols >> 1;
        while j < cols {
            let x = (i - (rows >> 1)) << 1;
            let y = (j - (cols >> 1)) << 1;
            for p in 4..8 {
                for k in 4..8 {
                    temp[i + p - 4][j + k - 4] = img_wav[x + p][y + k];
                }
            }
            j += 4;
        }
        i += 4;
    }
    // HL1
    i = 0;
    while i < (rows >> 1) {
        let mut j = cols >> 1;
        while j < cols {
            let x = i << 1;
            let y = (j - (cols >> 1)) << 1;
            for p in 0..4 {
                for k in 4..8 {
                    temp[i + p][j + k - 4] = img_wav[x + p][y + k];
                }
            }
            j += 4;
        }
        i += 4;
    }
    // LH1
    i = rows >> 1;
    while i < rows {
        let mut j = 0;
        while j < (cols >> 1) {
            let x = (i - (rows >> 1)) << 1;
            let y = j << 1;
            for p in 4..8 {
                for k in 0..4 {
                    temp[i + p - 4][j + k] = img_wav[x + p][y + k];
                }
            }
            j += 4;
        }
        i += 4;
    }
    // HH2
    i = rows >> 2;
    while i < (rows >> 1) {
        let mut j = cols >> 2;
        while j < (cols >> 1) {
            let x = (i - (rows >> 2)) << 2;
            let y = (j - (cols >> 2)) << 2;
            temp[i][j] = img_wav[x + 2][y + 2];
            temp[i][j + 1] = img_wav[x + 2][y + 3];
            temp[i + 1][j] = img_wav[x + 3][y + 2];
            temp[i + 1][j + 1] = img_wav[x + 3][y + 3];
            j += 2;
        }
        i += 2;
    }
    // HL2
    i = 0;
    while i < (rows >> 2) {
        let mut j = cols >> 2;
        while j < (cols >> 1) {
            let x = i << 2;
            let y = (j - (cols >> 2)) << 2;
            temp[i][j] = img_wav[x][y + 2];
            temp[i][j + 1] = img_wav[x][y + 3];
            temp[i + 1][j] = img_wav[x + 1][y + 2];
            temp[i + 1][j + 1] = img_wav[x + 1][y + 3];
            j += 2;
        }
        i += 2;
    }
    // LH2
    i = rows >> 2;
    while i < (rows >> 1) {
        let mut j = 0;
        while j < (cols >> 2) {
            let x = (i - (rows >> 2)) << 2;
            let y = j << 2;
            temp[i][j] = img_wav[x + 2][y];
            temp[i][j + 1] = img_wav[x + 2][y + 1];
            temp[i + 1][j] = img_wav[x + 3][y];
            temp[i + 1][j + 1] = img_wav[x + 3][y + 1];
            j += 2;
        }
        i += 2;
    }
    // HH3
    let x0 = rows >> 3;
    for i in (rows >> 3)..(rows >> 2) {
        for j in (cols >> 3)..(cols >> 2) {
            temp[i][j] = img_wav[((i - x0) << 3) + 1][((j - (cols >> 3)) << 3) + 1];
        }
    }
    // HL3
    for i in 0..(rows >> 3) {
        for j in (cols >> 3)..(cols >> 2) {
            temp[i][j] = img_wav[i << 3][((j - (cols >> 3)) << 3) + 1];
        }
    }
    // LH3
    for i in (rows >> 3)..(rows >> 2) {
        for j in 0..(cols >> 3) {
            temp[i][j] = img_wav[((i - x0) << 3) + 1][j << 3];
        }
    }
    // LL3
    for i in 0..(rows >> 3) {
        for j in 0..(cols >> 3) {
            temp[i][j] = img_wav[i << 3][j << 3];
        }
    }
    for i in 0..rows {
        for j in 0..cols {
            img_wav[i][j] = temp[i][j];
        }
    }
}

/// Generic CoeffRegroup body (temp buffer then copy-back).
fn regroup_generic<T: Copy + Default>(transformed: &mut Vec<Vec<T>>, rows: usize, cols: usize) {
    let mut temp = alloc_temp::<T>(rows, cols);
    let mut i = rows >> 1;
    while i < rows {
        let mut j = cols >> 1;
        while j < cols {
            let x = (i - (rows >> 1)) << 1;
            let y = (j - (cols >> 1)) << 1;
            for p in 4..8 {
                for k in 4..8 {
                    temp[x + p][y + k] = transformed[i + p - 4][j + k - 4];
                }
            }
            j += 4;
        }
        i += 4;
    }
    i = 0;
    while i < (rows >> 1) {
        let mut j = cols >> 1;
        while j < cols {
            let x = i << 1;
            let y = (j - (cols >> 1)) << 1;
            for p in 0..4 {
                for k in 4..8 {
                    temp[x + p][y + k] = transformed[i + p][j + k - 4];
                }
            }
            j += 4;
        }
        i += 4;
    }
    i = rows >> 1;
    while i < rows {
        let mut j = 0;
        while j < (cols >> 1) {
            let x = (i - (rows >> 1)) << 1;
            let y = j << 1;
            for p in 4..8 {
                for k in 0..4 {
                    temp[x + p][y + k] = transformed[i + p - 4][j + k];
                }
            }
            j += 4;
        }
        i += 4;
    }
    i = rows >> 2;
    while i < (rows >> 1) {
        let mut j = cols >> 2;
        while j < (cols >> 1) {
            let x = (i - (rows >> 2)) << 2;
            let y = (j - (cols >> 2)) << 2;
            temp[x + 2][y + 2] = transformed[i][j];
            temp[x + 2][y + 3] = transformed[i][j + 1];
            temp[x + 3][y + 2] = transformed[i + 1][j];
            temp[x + 3][y + 3] = transformed[i + 1][j + 1];
            j += 2;
        }
        i += 2;
    }
    i = 0;
    while i < (rows >> 2) {
        let mut j = cols >> 2;
        while j < (cols >> 1) {
            let x = i << 2;
            let y = (j - (cols >> 2)) << 2;
            temp[x][y + 2] = transformed[i][j];
            temp[x][y + 3] = transformed[i][j + 1];
            temp[x + 1][y + 2] = transformed[i + 1][j];
            temp[x + 1][y + 3] = transformed[i + 1][j + 1];
            j += 2;
        }
        i += 2;
    }
    i = rows >> 2;
    while i < (rows >> 1) {
        let mut j = 0;
        while j < (cols >> 2) {
            let x = (i - (rows >> 2)) << 2;
            let y = j << 2;
            temp[x + 2][y] = transformed[i][j];
            temp[x + 2][y + 1] = transformed[i][j + 1];
            temp[x + 3][y] = transformed[i + 1][j];
            temp[x + 3][y + 1] = transformed[i + 1][j + 1];
            j += 2;
        }
        i += 2;
    }
    let x0 = rows >> 3;
    for i in (rows >> 3)..(rows >> 2) {
        for j in (cols >> 3)..(cols >> 2) {
            temp[((i - x0) << 3) + 1][((j - (cols >> 3)) << 3) + 1] = transformed[i][j];
        }
    }
    for i in 0..(rows >> 3) {
        for j in (cols >> 3)..(cols >> 2) {
            temp[i << 3][((j - (cols >> 3)) << 3) + 1] = transformed[i][j];
        }
    }
    for i in (rows >> 3)..(rows >> 2) {
        for j in 0..(cols >> 3) {
            temp[((i - x0) << 3) + 1][j << 3] = transformed[i][j];
        }
    }
    for i in 0..(rows >> 3) {
        for j in 0..(cols >> 3) {
            temp[i << 3][j << 3] = transformed[i][j];
        }
    }
    for i in 0..rows {
        for j in 0..cols {
            transformed[i][j] = temp[i][j];
        }
    }
}

pub fn coeff_degroup(img_wav: &mut ImageI32, rows: usize, cols: usize) {
    degroup_generic(img_wav, rows, cols);
}

pub fn coeff_degroup_floating(img_wav: &mut ImageF32, rows: usize, cols: usize) {
    degroup_generic(img_wav, rows, cols);
}

pub fn coeff_regroup(transformed: &mut ImageI32, rows: usize, cols: usize) {
    regroup_generic(transformed, rows, cols);
}

pub fn coeff_regroup_f97(transformed: &mut ImageF32, rows: usize, cols: usize) {
    regroup_generic(transformed, rows, cols);
}
