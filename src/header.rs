//! Header encode/decode — original/source/header.c

use crate::bitstream::{bits_output, bits_read};
use crate::error::{BpeError, BpeResult};
use crate::types::{CodingPara, Header, INTEGER_WAVELET};

fn write_part1(coding: &mut CodingPara) -> BpeResult<()> {
    bits_output(coding, coding.header.part1.start_img_flag as u32, 1)?;
    bits_output(coding, coding.header.part1.eng_img_flg as u32, 1)?;
    bits_output(coding, coding.header.part1.segment_count_8bits as u32, 8)?;
    bits_output(coding, coding.header.part1.bit_depth_dc_5bits as u32, 5)?;
    bits_output(coding, coding.header.part1.bit_depth_ac_5bits as u32, 5)?;
    bits_output(coding, coding.header.part1.reserved as u32, 1)?;
    bits_output(coding, coding.header.part1.part2_flag as u32, 1)?;
    bits_output(coding, coding.header.part1.part3_flag as u32, 1)?;
    bits_output(coding, coding.header.part1.part4_flag as u32, 1)?;

    if coding.header.part1.eng_img_flg {
        bits_output(coding, coding.header.part1.pad_rows_3bits as u32, 3)?;
        bits_output(coding, coding.header.part1.reserved_5bits as u32, 5)?;
    }
    Ok(())
}

fn write_part2(coding: &mut CodingPara) -> BpeResult<()> {
    bits_output(coding, coding.header.part2.seg_byte_limit_27bits, 27)?;
    bits_output(coding, coding.header.part2.dc_stop as u32, 1)?;
    bits_output(coding, coding.header.part2.bit_plane_stop_5bits as u32, 5)?;
    bits_output(coding, coding.header.part2.stage_stop_2bits as u32, 2)?;
    bits_output(coding, coding.header.part2.use_fill as u32, 1)?;
    bits_output(coding, coding.header.part2.reserved_4bits as u32, 4)?;
    Ok(())
}

fn write_part3(coding: &mut CodingPara) -> BpeResult<()> {
    bits_output(coding, coding.header.part3.s_20bits, 20)?;
    bits_output(coding, coding.header.part3.opt_dc_select as u32, 1)?;
    bits_output(coding, coding.header.part3.opt_ac_select as u32, 1)?;
    bits_output(coding, coding.header.part3.reserved_2bits as u32, 2)?;
    Ok(())
}

fn write_part4(coding: &mut CodingPara) -> BpeResult<()> {
    bits_output(coding, coding.header.part4.dwt_type as u32, 1)?;
    bits_output(coding, coding.header.part4.reserved_2bits as u32, 2)?;
    bits_output(coding, coding.header.part4.signed_pixels as u32, 1)?;
    bits_output(coding, coding.header.part4.pixel_bit_depth_4bits as u32, 4)?;
    bits_output(coding, coding.header.part4.image_width_20bits, 20)?;
    bits_output(coding, coding.header.part4.transpose_img as u32, 1)?;
    bits_output(coding, coding.header.part4.codeword_length_2bits as u32, 2)?;
    bits_output(coding, coding.header.part4.reserved as u32, 1)?;
    bits_output(coding, coding.header.part4.custom_wt_flag as u32, 1)?;
    if !coding.header.part4.custom_wt_flag {
        bits_output(coding, 0, 8)?;
        bits_output(coding, 0, 8)?;
        bits_output(coding, 0, 4)?;
    } else {
        bits_output(coding, coding.header.part4.custom_wt_hh1 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_hl1 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_lh1 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_hh2 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_hl2 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_lh2 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_hh3 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_hl3 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_lh3 as u32, 2)?;
        bits_output(coding, coding.header.part4.custom_wt_ll3 as u32, 2)?;
    }
    bits_output(coding, coding.header.part4.reserved_11bits as u32, 11)?;
    Ok(())
}

pub fn header_output(coding: &mut CodingPara) -> BpeResult<()> {
    write_part1(coding)?;

    if coding.header.part1.part2_flag {
        write_part2(coding)?;
    }

    if coding.header.part1.part3_flag {
        write_part3(coding)?;
    }

    if coding.header.part1.part4_flag {
        write_part4(coding)?;
    }
    Ok(())
}

fn read_part1(coding: &mut CodingPara) -> BpeResult<()> {
    coding.header.part1.start_img_flag = bits_read(coding, 1)? != 0;
    coding.header.part1.eng_img_flg = bits_read(coding, 1)? != 0;
    coding.header.part1.segment_count_8bits = bits_read(coding, 8)? as u8;
    coding.header.part1.bit_depth_dc_5bits = bits_read(coding, 5)? as u8;
    coding.header.part1.bit_depth_ac_5bits = bits_read(coding, 5)? as u8;
    coding.header.part1.reserved = bits_read(coding, 1)? != 0;
    coding.header.part1.part2_flag = bits_read(coding, 1)? != 0;
    coding.header.part1.part3_flag = bits_read(coding, 1)? != 0;
    coding.header.part1.part4_flag = bits_read(coding, 1)? != 0;

    if coding.header.part1.eng_img_flg {
        coding.header.part1.pad_rows_3bits = bits_read(coding, 3)? as u8;
        coding.header.part1.reserved_5bits = bits_read(coding, 5)? as u8;
    }
    Ok(())
}

fn read_part2(coding: &mut CodingPara) -> BpeResult<()> {
    coding.header.part2.seg_byte_limit_27bits = bits_read(coding, 27)?;
    coding.header.part2.dc_stop = bits_read(coding, 1)? != 0;
    coding.header.part2.bit_plane_stop_5bits = bits_read(coding, 5)? as u8;
    coding.header.part2.stage_stop_2bits = bits_read(coding, 2)? as u8;
    coding.header.part2.use_fill = bits_read(coding, 1)? != 0;
    coding.header.part2.reserved_4bits = bits_read(coding, 4)? as u8;
    Ok(())
}

fn read_part3(coding: &mut CodingPara) -> BpeResult<()> {
    coding.header.part3.s_20bits = bits_read(coding, 20)?;
    if coding.bits_per_pixel != 0.0 {
        coding.decoding_allowed_bits_size_in_segment =
            (coding.bits_per_pixel * coding.header.part3.s_20bits as f32 * 64.0) as u32;
        let seg_bits = coding.header.part2.seg_byte_limit_27bits << 3;
        if coding.decoding_allowed_bits_size_in_segment > seg_bits {
            coding.decoding_allowed_bits_size_in_segment = seg_bits;
        }
    } else {
        coding.decoding_allowed_bits_size_in_segment = 0;
        if coding.header.part2.seg_byte_limit_27bits != 0 {
            coding.decoding_allowed_bits_size_in_segment =
                coding.header.part2.seg_byte_limit_27bits << 3;
        }
    }
    coding.header.part3.opt_dc_select = bits_read(coding, 1)? != 0;
    coding.header.part3.opt_ac_select = bits_read(coding, 1)? != 0;
    coding.header.part3.reserved_2bits = bits_read(coding, 2)? as u8;
    Ok(())
}

fn read_part4(coding: &mut CodingPara) -> BpeResult<()> {
    coding.header.part4.dwt_type = bits_read(coding, 1)? as u8;
    coding.header.part4.reserved_2bits = bits_read(coding, 2)? as u8;
    coding.header.part4.signed_pixels = bits_read(coding, 1)? != 0;
    coding.header.part4.pixel_bit_depth_4bits = bits_read(coding, 4)? as u8;
    coding.header.part4.image_width_20bits = bits_read(coding, 20)?;
    coding.header.part4.transpose_img = bits_read(coding, 1)? as u8;
    coding.header.part4.codeword_length_2bits = bits_read(coding, 2)? as u8;
    coding.bits.code_word_length = match coding.header.part4.codeword_length_2bits {
        0 => 8,
        1 => 16,
        2 => 24,
        3 => 32,
        _ => 8,
    };
    coding.header.part4.reserved = bits_read(coding, 1)? != 0;
    let custom = bits_read(coding, 1)?;
    coding.header.part4.custom_wt_flag = custom != 0;
    if custom != 0 {
        coding.header.part4.custom_wt_hh1 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_hl1 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_lh1 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_hh2 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_hl2 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_lh2 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_hh3 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_hl3 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_lh3 = bits_read(coding, 2)? as u8;
        coding.header.part4.custom_wt_ll3 = bits_read(coding, 2)? as u8;
    } else {
        let _ = bits_read(coding, 20)?;
    }
    coding.header.part4.reserved_11bits = bits_read(coding, 11)? as u16;
    Ok(())
}

pub fn header_readin(coding: &mut CodingPara) -> BpeResult<()> {
    read_part1(coding)?;

    if coding.header.part1.part2_flag {
        read_part2(coding)?;
    }

    if coding.header.part1.part3_flag {
        read_part3(coding)?;
    } else {
        coding.header.part2.seg_byte_limit_27bits = 0;
    }

    if coding.header.part1.part4_flag {
        read_part4(coding)?;
    }

    coding.decoding_stop_locations.bit_plane_stop_decoding = -1;
    coding.decoding_stop_locations.block_no_stop_decoding = -1;
    coding.decoding_stop_locations.location_find = false;
    coding.decoding_stop_locations.x_location_stop_decoding = -1;
    coding.decoding_stop_locations.y_location_stop_decoding = -1;
    Ok(())
}

pub fn header_update(header: &mut Header) -> BpeResult<()> {
    if header.part1.start_img_flag {
        header.part1.start_img_flag = false;
    }
    header.part1.bit_depth_ac_5bits = 0;
    header.part1.bit_depth_dc_5bits = 0;
    header.part1.segment_count_8bits = header.part1.segment_count_8bits.wrapping_add(1);
    header.part1.part2_flag = false;
    header.part1.part3_flag = false;
    header.part1.part4_flag = false;
    if header.part1.part2_flag {
        header.part2.seg_byte_limit_27bits = 1_000_000;
        header.part2.bit_plane_stop_5bits = header.part2.bit_plane_stop_5bits.wrapping_add(1);
        header.part2.stage_stop_2bits = header.part2.stage_stop_2bits.wrapping_add(1);
        header.part2.use_fill = !header.part2.use_fill;
    }
    if header.part1.part3_flag {
        header.part3.s_20bits = 1000;
        header.part3.opt_dc_select = !header.part3.opt_dc_select;
        header.part3.opt_ac_select = !header.part3.opt_ac_select;
    }
    if header.part1.part4_flag {
        header.part4.dwt_type = INTEGER_WAVELET;
    }
    if header.part3.s_20bits < 16 {
        return Err(BpeError::InvalidHeader);
    }
    Ok(())
}
