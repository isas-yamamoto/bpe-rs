//! DPCM mapping/demapping for DC coefficients - original/source/DC_EnDeCoding.c

use crate::types::BitPlaneBits;

pub fn dpcm_dc_mapper(block_info: &mut [BitPlaneBits], size: usize, n: i16) {
    let x_min: i32 = -(1 << (n - 1));
    let x_max: i32 = (1i32 << (n - 1)) - 1;
    let mut max_mapped: u32 = 0;

    let mut diff_dc = vec![0i32; size];

    block_info[0].mapped_dc = block_info[0].shifted_dc;

    let mut bits1: i32 = 0;
    for _ in 0..(n - 1) {
        bits1 = (bits1 << 1) + 1;
    }

    let sd0 = block_info[0].shifted_dc as i32;
    if (sd0 & (1 << (n - 1))) > 0 {
        block_info[0].shifted_dc = (-(((sd0 ^ bits1) & bits1) + 1)) as u32;
    }
    diff_dc[0] = block_info[0].shifted_dc as i32;

    for i in 1..size {
        let sdi = block_info[i].shifted_dc as i32;
        if (sdi & (1 << (n - 1))) > 0 {
            block_info[i].shifted_dc = (-(((sdi ^ bits1) & bits1) + 1)) as u32;
        }
        diff_dc[i] = (block_info[i].shifted_dc as i32) - (block_info[i - 1].shifted_dc as i32);
    }

    for i in 1..size {
        let prev = block_info[i - 1].shifted_dc as i32;
        let theta = (prev - x_min).min(x_max - prev);
        if diff_dc[i] >= 0 && diff_dc[i] <= theta {
            block_info[i].mapped_dc = (2 * diff_dc[i]) as u32;
        } else if diff_dc[i] < 0 && diff_dc[i] >= -theta {
            block_info[i].mapped_dc = (-2 * diff_dc[i] - 1) as u32;
        } else {
            block_info[i].mapped_dc = (theta + diff_dc[i].abs()) as u32;
        }
        if block_info[i].mapped_dc > max_mapped {
            max_mapped = block_info[i].mapped_dc;
        }
    }
}

pub fn dpcm_dc_demapper(block_info: &mut [BitPlaneBits], size: usize, n: i16) {
    let x_max: i32 = (1i32 << (n - 1)) - 1;
    let x_min: i32 = -(1 << (n - 1));

    let mut diff_dc = vec![0i32; size];

    block_info[0].shifted_dc = block_info[0].mapped_dc;
    diff_dc[0] = block_info[0].shifted_dc as i32;

    let mut bits1: i32 = 0;
    for _ in 0..(n - 1) {
        bits1 = (bits1 << 1) + 1;
    }

    let sd0 = block_info[0].shifted_dc as i32;
    if (sd0 & (1 << (n - 1))) > 0 {
        block_info[0].shifted_dc = (-(((sd0 ^ bits1) & bits1) + 1)) as u32;
    }

    for i in 1..size {
        let prev = block_info[i - 1].shifted_dc as i32;
        let theta = (prev - x_min).min(x_max - prev);
        let mapped = block_info[i].mapped_dc as i32;

        let mut d;
        if mapped > 2 * theta {
            if prev < 0 {
                d = mapped - theta;
            } else {
                d = theta - mapped;
            }
        } else if mapped % 2 == 0 {
            d = mapped / 2;
        } else {
            d = -((mapped + 1) / 2);
        }
        diff_dc[i] = d;
        block_info[i].shifted_dc = (d + prev) as u32;
        let _ = &mut d;
    }
}
