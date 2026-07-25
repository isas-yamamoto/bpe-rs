//! Encoder engine — original/source/bpe_encoder.c
//!
//! Pipeline:
//!   1. size / pad  2. read image  3. dwt_forward  4. build_block_string
//!   5. per segment: prepare_last_segment_header -> dc_encoding -> ac_bpe_encoding
//!   6. flush

use crate::ac::ac_bpe_encoding;
use crate::bitstream::segment_buffer_flush_encoder;
use crate::dc::dc_encoding;
use crate::error::BpeResult;
use crate::header::header_update;
use crate::image_io::{image_read, image_size};
use crate::types::{
    alloc_block_string, alloc_image_i32, BitPlaneBits, BlockString, CodingPara, ImageI32,
    BLOCK_SIZE,
};
use crate::wavelet::dwt_forward;

/// Reorganize the transformed image into the block string layout
/// (each 8x8 block occupies 8 consecutive rows of length 8).
fn build_block_string(
    transformed: &ImageI32,
    image_rows: usize,
    image_width: usize,
    block_string: &mut BlockString,
) {
    let block_row = image_rows / BLOCK_SIZE;
    let block_col = image_width / BLOCK_SIZE;
    let mut counter = 0usize;
    for i in 0..block_row {
        for j in 0..block_col {
            for k in 0..BLOCK_SIZE {
                for p in 0..BLOCK_SIZE {
                    block_string[counter][p] = transformed[i * BLOCK_SIZE + k][j * BLOCK_SIZE + p];
                }
                counter += 1;
            }
        }
    }
}

/// Update header flags when this segment reaches or overflows the last blocks.
fn prepare_last_segment_header(coding: &mut CodingPara, total_blocks: u32, temp_padded_rows: u8) {
    let s = coding.header.part3.s_20bits;
    if coding.block_counter + s == total_blocks {
        coding.header.part1.eng_img_flg = true;
        coding.header.part1.pad_rows_3bits = temp_padded_rows;
    } else if coding.block_counter + s > total_blocks {
        // Last packet holds fewer blocks than requested; re-enable part2/part3.
        coding.header.part1.eng_img_flg = true;
        coding.header.part1.pad_rows_3bits = temp_padded_rows;
        coding.header.part1.part3_flag = true;
        coding.header.part3.s_20bits = total_blocks - coding.block_counter;
        coding.header.part1.part2_flag = true;
        coding.header.part2.seg_byte_limit_27bits =
            (coding.bits_per_pixel * coding.header.part3.s_20bits as f32 * 64.0 / 8.0) as u32;
    }
}

/// Full pipeline: wavelet transform through the per-segment DC/AC
/// bit-plane coding loop and the final buffer flush.
pub fn encoder_engine(coding: &mut CodingPara) -> BpeResult<()> {
    // 1. Validate / determine the image dimensions from the file size.
    image_size(coding)?;

    // Determine how many rows must be replicated to reach a multiple of 8.
    if coding.image_rows % BLOCK_SIZE as u32 != 0 {
        coding.header.part1.pad_rows_3bits =
            (BLOCK_SIZE as u32 - (coding.image_rows % BLOCK_SIZE as u32)) as u8;
    }

    coding.header.part4.image_width_20bits = coding.image_width;

    if coding.image_width % BLOCK_SIZE as u32 != 0 {
        coding.pad_cols_3bits =
            (BLOCK_SIZE as u32 - (coding.image_width % BLOCK_SIZE as u32)) as u8;
    }

    let rows = coding.image_rows as usize;
    let width = coding.image_width as usize;
    let pad_rows = rows + coding.header.part1.pad_rows_3bits as usize;
    let pad_cols = width + coding.pad_cols_3bits as usize;

    // 2. Read the original image and build the padded (replicated) image.
    let unpadded = image_read(coding)?;
    let mut original = alloc_image_i32(pad_rows, pad_cols);
    for r in 0..rows {
        for c in 0..width {
            original[r][c] = unpadded[r][c];
        }
    }
    // Replicate the last rows.
    for i in 0..coding.header.part1.pad_rows_3bits as usize {
        for j in 0..pad_cols {
            original[i + rows][j] = original[rows - 1][j];
        }
    }
    // Replicate the last columns.
    for i in 0..coding.pad_cols_3bits as usize {
        for j in 0..pad_rows {
            original[j][i + width] = original[j][width - 1];
        }
    }

    // 3. Open the output bitstream.
    let out = coding.coding_output_file.clone();
    coding.bits.open_write(&out)?;

    // 4. Forward DWT, then build the block string.
    let mut transformed = alloc_image_i32(pad_rows, pad_cols);
    dwt_forward(coding, &original, &mut transformed)?;

    let total_blocks = ((pad_rows / BLOCK_SIZE) * (pad_cols / BLOCK_SIZE)) as u32;
    let mut block_string = alloc_block_string(total_blocks as usize);
    build_block_string(&transformed, pad_rows, pad_cols, &mut block_string);

    // 5. Per-segment DC/AC bit-plane coding loop.
    let temp_padded_rows = coding.header.part1.pad_rows_3bits;
    coding.header.part1.pad_rows_3bits = 0;

    while coding.block_counter < total_blocks {
        prepare_last_segment_header(coding, total_blocks, temp_padded_rows);

        let seg = coding.header.part3.s_20bits as usize;
        let mut block_info: Vec<BitPlaneBits> = (0..seg).map(|_| BitPlaneBits::default()).collect();

        dc_encoding(coding, &block_string, &mut block_info)?;

        if !coding.segment_full && !(coding.header.part2.dc_stop && coding.header.part1.part2_flag)
        {
            ac_bpe_encoding(coding, &mut block_info)?;
        }

        if coding.header.part1.eng_img_flg {
            break;
        }

        coding.block_counter += coding.header.part3.s_20bits;
        header_update(&mut coding.header)?;
        segment_buffer_flush_encoder(coding)?;
        coding.segment_full = false;
    }

    // 6. Final flush.
    segment_buffer_flush_encoder(coding)?;
    Ok(())
}
