//! Decoder engine — original/source/bpe_decoder.c
//!
//! Pipeline per segment: header -> dc_decoding -> ac_bpe_decoding -> adjust_output.
//! Then reassemble coefficient image and inverse DWT.

use crate::ac::ac_bpe_decoding;
use crate::adjust::adjust_output;
use crate::bitstream::segment_buffer_flush_decoder;
use crate::dc::dc_decoding;
use crate::error::BpeResult;
use crate::header::header_readin;
use crate::image_io::{image_write, image_write_float};
use crate::types::{
    alloc_block_string, alloc_image_f32, alloc_image_i32, BitPlaneBits, BlockString, CodingPara,
    ImageF32, ImageI32, BLOCK_SIZE, INTEGER_WAVELET, TRANSPOSE,
};
use crate::wavelet::{coeff_degroup, coeff_degroup_floating, dwt_reverse, dwt_reverse_floating};

/// Per-segment decoded coefficient storage (integer and floating paths).
struct SegmentBlocks {
    freq: BlockString,
    floatv: Vec<[f32; BLOCK_SIZE]>,
    blocks: usize,
}

/// Degroup, inverse DWT, optional transpose, write (integer path).
fn decoding_output_integer(coding: &mut CodingPara, img: &mut ImageI32) -> BpeResult<()> {
    let rows = coding.image_rows as usize;
    let cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;
    coeff_degroup(img, rows, cols);
    dwt_reverse(img, coding)?;

    if coding.header.part4.transpose_img == TRANSPOSE {
        let width = coding.image_width as usize;
        let mut transposed = alloc_image_i32(rows, width);
        for i in 0..rows {
            for j in 0..width {
                transposed[j][i] = img[i][j];
            }
        }
        image_write(coding, &transposed)?;
    } else {
        let snapshot = img.clone();
        image_write(coding, &snapshot)?;
    }
    Ok(())
}

/// Degroup, inverse DWT, optional transpose, write (floating path).
fn decoding_output_floating(coding: &mut CodingPara, img: &mut ImageF32) -> BpeResult<()> {
    let rows = coding.image_rows as usize;
    let cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;
    coeff_degroup_floating(img, rows, cols);
    dwt_reverse_floating(img, coding)?;

    if coding.header.part4.transpose_img == TRANSPOSE {
        let width = coding.image_width as usize;
        let mut transposed = alloc_image_f32(rows, width);
        for i in 0..rows {
            for j in 0..width {
                transposed[j][i] = img[i][j];
            }
        }
        image_write_float(coding, &transposed)?;
    } else {
        image_write_float(coding, img)?;
    }
    Ok(())
}

/// Decode one segment: DC/AC/adjust, copy coefficients back, flush, reset state.
fn decode_one_segment(coding: &mut CodingPara) -> BpeResult<SegmentBlocks> {
    let s = coding.header.part3.s_20bits as usize;

    let mut freq = alloc_block_string(s);
    let mut floatv = vec![[0.0f32; BLOCK_SIZE]; s * BLOCK_SIZE];
    let mut block_info: Vec<BitPlaneBits> = (0..s).map(|_| BitPlaneBits::default()).collect();

    dc_decoding(coding, &freq, &floatv, &mut block_info)?;
    ac_bpe_decoding(coding, &mut block_info)?;
    adjust_output(coding, &mut block_info)?;

    // Copy the decoded coefficients back into the segment storage (in C this
    // happens through the aliased PtrBlockAddress pointers).
    for i in 0..s {
        for r in 0..BLOCK_SIZE {
            freq[i * BLOCK_SIZE + r] = block_info[i].block_int[r];
            floatv[i * BLOCK_SIZE + r] = block_info[i].block_float[r];
        }
    }

    segment_buffer_flush_decoder(coding)?;
    coding.segment_full = false;
    coding.rate_reached = false;
    coding.decoding_stop_locations.bit_plane_stop_decoding = 0;
    coding.block_counter += coding.header.part3.s_20bits;

    Ok(SegmentBlocks {
        freq,
        floatv,
        blocks: s,
    })
}

/// Reassemble per-segment coefficient storage into full int/float images.
fn reassemble_images(
    segments: &[SegmentBlocks],
    rows: usize,
    pad_cols: usize,
    width: usize,
) -> (ImageI32, ImageF32) {
    let mut img_int = alloc_image_i32(rows, pad_cols);
    let mut img_float = alloc_image_f32(rows, pad_cols);

    let mut x = 0usize;
    let mut y = 0usize;
    for seg in segments {
        let mut f_x = 0usize;
        loop {
            for i in 0..BLOCK_SIZE {
                for j in 0..BLOCK_SIZE {
                    img_int[x + i][y + j] = seg.freq[f_x + i][j];
                    img_float[x + i][y + j] = seg.floatv[f_x + i][j];
                }
            }
            y += BLOCK_SIZE;
            if y >= width {
                y = 0;
                x += BLOCK_SIZE;
            }
            f_x += BLOCK_SIZE;
            if f_x >= seg.blocks * BLOCK_SIZE {
                break;
            }
        }
    }

    (img_int, img_float)
}

/// Read the bitstream segment by segment, decode each, reassemble the
/// coefficient image, and inverse transform to the output image.
pub fn decoder_engine(coding: &mut CodingPara) -> BpeResult<()> {
    let input = coding.input_file.clone();
    coding.bits.open_read(&input)?;

    header_readin(coding)?;
    coding.image_width = coding.header.part4.image_width_20bits;

    if coding.image_width % BLOCK_SIZE as u32 != 0 {
        coding.pad_cols_3bits =
            (BLOCK_SIZE as u32 - (coding.image_width % BLOCK_SIZE as u32)) as u8;
    } else {
        coding.pad_cols_3bits = 0;
    }

    let mut segments: Vec<SegmentBlocks> = Vec::new();
    let mut total_blocks: usize = 0;

    loop {
        total_blocks += coding.header.part3.s_20bits as usize;
        segments.push(decode_one_segment(coding)?);

        if coding.header.part1.eng_img_flg {
            break;
        }
        header_readin(coding)?;
    }

    // Reassemble the full coefficient image (integer and floating variants).
    coding.image_rows =
        (total_blocks * 64 / (coding.image_width + coding.pad_cols_3bits as u32) as usize) as u32;

    let rows = coding.image_rows as usize;
    let pad_cols = (coding.image_width + coding.pad_cols_3bits as u32) as usize;
    let width = coding.image_width as usize;

    let (mut img_int, mut img_float) = reassemble_images(&segments, rows, pad_cols, width);

    if coding.header.part4.dwt_type == INTEGER_WAVELET {
        decoding_output_integer(coding, &mut img_int)?;
    } else {
        decoding_output_floating(coding, &mut img_float)?;
    }
    Ok(())
}
