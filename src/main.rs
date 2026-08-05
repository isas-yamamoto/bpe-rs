//! CLI — original/source/main.c

use bpe_rs::decoder::decoder_engine;
use bpe_rs::encoder::encoder_engine;
use bpe_rs::error::{error_exit, BpeError};
use bpe_rs::types::{CodingPara, IMAGE_ROWS_MIN, IMAGE_WIDTH_MAX, IMAGE_WIDTH_MIN, SEGMENT_S_MIN};

fn usage() {
    eprintln!("/******************   Bit Plane Encoder Using Wavelet Transform    ************/");
    eprintln!("Last modified on March 9, 2008 (Rust CLI)");
    eprintln!("bpe [-e]|[-d] [Input_file_name] [-o Output_file_name] [-r BitsPerPixel]");
    eprintln!("\nParameters: ");
    eprintln!("[-e]: encoding filename; ");
    eprintln!("[-d]: decoding filename; ");
    eprintln!("[-o]: provide ouput file name. ");
    eprintln!("[-r]: bits per pixel for encoding.");
    eprintln!("[-w]: the number of pixels of each row. ");
    eprintln!("[-h]: the number of pixels of each column. ");
    eprintln!("[-b]: the number of bits of each pixel. By default it is 8.");
    eprintln!("[-f]: byte order of a pixel (0 little endian, 1 big endian).");
    eprintln!("[-t]: wavelet transform. 1 integer 9-7, 0 floating 9-7.");
    eprintln!("[-s]: the number of blocks in each segment. By default it is 256.");
    eprintln!("[--compat-c-ref]: reproduce the C reference implementation's integer");
    eprintln!("    and float quirks bit-for-bit (float 9/7 inverse-lifting rounding,");
    eprintln!("    DPCM DC mapping's int16 overflow). Off by default, which uses");
    eprintln!("    the corrected/more precise behavior instead -- bitstreams produced");
    eprintln!("    without this flag are not guaranteed to interoperate with the C");
    eprintln!("    reference implementation.");
    eprintln!("eg 1: bpe -e sensin.img -o codes -r 1.0 -w 256 -h 256 -s 256");
    eprintln!("eg 2: bpe -d codes -o ss.img");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut encode = false;
    let mut decode = false;
    let mut input = String::new();
    let mut output = String::new();
    let mut bpp: f32 = 0.0;
    let mut rows: u32 = 0;
    let mut width: u32 = 0;
    let mut bit_depth: u8 = 8;
    let mut byte_order: u8 = 0;
    let mut dwt_type: u8 = 1;
    let mut signed_pixels: u8 = 0;
    let mut segment: u32 = 256;
    let mut strict_c_compat = false;

    let mut i = 1usize;
    while i < args.len() {
        let a = args[i].as_str();
        let need = |i: usize, args: &[String]| -> String {
            if i + 1 >= args.len() {
                usage();
                error_exit(BpeError::InvalidCodingParameters);
            }
            args[i + 1].clone()
        };
        match a {
            "-e" => {
                encode = true;
                input = need(i, &args);
                i += 1;
            }
            "-d" => {
                decode = true;
                input = need(i, &args);
                i += 1;
            }
            "-o" => {
                output = need(i, &args);
                i += 1;
            }
            "-r" => {
                bpp = need(i, &args).parse().unwrap_or_else(|_| {
                    usage();
                    error_exit(BpeError::InvalidCodingParameters);
                });
                i += 1;
            }
            "-h" => {
                rows = need(i, &args).parse().unwrap_or_else(|_| {
                    usage();
                    error_exit(BpeError::InvalidCodingParameters);
                });
                i += 1;
            }
            "-w" => {
                width = need(i, &args).parse().unwrap_or_else(|_| {
                    usage();
                    error_exit(BpeError::InvalidCodingParameters);
                });
                i += 1;
            }
            "-f" => {
                byte_order = need(i, &args).parse().unwrap_or(0);
                i += 1;
            }
            "-b" => {
                let v: u32 = need(i, &args).parse().unwrap_or(8);
                bit_depth = (v % 16) as u8;
                i += 1;
            }
            "-t" => {
                dwt_type = need(i, &args).parse().unwrap_or(1);
                i += 1;
            }
            "-g" => {
                let v: u8 = need(i, &args).parse().unwrap_or(0);
                signed_pixels = if v > 0 { 1 } else { 0 };
                i += 1;
            }
            "-s" => {
                segment = need(i, &args).parse().unwrap_or(256);
                i += 1;
            }
            "--compat-c-ref" => {
                strict_c_compat = true;
            }
            _ => {
                usage();
                error_exit(BpeError::InvalidCodingParameters);
            }
        }
        i += 1;
    }

    if (encode && decode) || (!encode && !decode) || input.is_empty() || output.is_empty() {
        usage();
        error_exit(BpeError::InvalidCodingParameters);
    }

    if encode {
        if width != 0 && (width < IMAGE_WIDTH_MIN || width > IMAGE_WIDTH_MAX) {
            error_exit(BpeError::InvalidCodingParameters);
        }
        if rows != 0 && rows < IMAGE_ROWS_MIN {
            error_exit(BpeError::InvalidCodingParameters);
        }
        if segment < SEGMENT_S_MIN {
            error_exit(BpeError::InvalidCodingParameters);
        }
        if !(0.0..=16.0).contains(&bpp) {
            error_exit(BpeError::InvalidCodingParameters);
        }

        // Mirror c_bridge/bridge.c field setup.
        let mut coding = CodingPara::new();
        coding.input_file = input;
        coding.coding_output_file = output;
        coding.bits_per_pixel = bpp;
        coding.image_rows = rows;
        coding.image_width = width;
        coding.pixel_byte_order = byte_order;
        coding.header.part4.pixel_bit_depth_4bits = bit_depth % 16;
        coding.header.part4.dwt_type = dwt_type;
        coding.header.part4.signed_pixels = signed_pixels != 0;
        coding.header.part3.s_20bits = segment;
        coding.strict_c_compat = strict_c_compat;
        if coding.bits_per_pixel != 0.0 && coding.header.part2.seg_byte_limit_27bits == 0 {
            coding.header.part2.seg_byte_limit_27bits =
                (coding.bits_per_pixel * coding.header.part3.s_20bits as f32 * 64.0 / 8.0) as u32;
        }

        if let Err(e) = encoder_engine(&mut coding) {
            error_exit(e);
        }
    } else {
        if bpp < 0.0 {
            error_exit(BpeError::InvalidCodingParameters);
        }

        let mut coding = CodingPara::new();
        coding.input_file = input;
        coding.coding_output_file = output;
        coding.bits_per_pixel = bpp;
        coding.pixel_byte_order = byte_order;
        coding.strict_c_compat = strict_c_compat;

        if let Err(e) = decoder_engine(&mut coding) {
            error_exit(e);
        }
    }
}
