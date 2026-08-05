//! Types — original/source/global.h

use crate::bitstream::BitStream;

pub const BUFFER_LENGTH: usize = 150;
pub const MAX_SYMBOLS_IN_BLOCK: usize = 22;
pub const INTEGER_WAVELET: u8 = 1;
pub const FLOAT_WAVELET: u8 = 0;
pub const GAGGLE_SIZE: usize = 16;
pub const NOTRANSPOSE: u8 = 0;
pub const TRANSPOSE: u8 = 1;
pub const BLOCK_SIZE: usize = 8;
pub const NEGATIVE_SIGN: u8 = 1;
pub const POSITIVE_SIGN: u8 = 0;

pub const IMAGE_WIDTH_MAX: u32 = 1 << 20;
pub const IMAGE_WIDTH_MIN: u32 = 17;
pub const IMAGE_ROWS_MIN: u32 = 17;
pub const SEGMENT_S_MIN: u32 = 16;
pub const SEGMENT_S_MAX: u32 = 1 << 20;
pub const SEGBYTE_MAX: u32 = 1 << 27;
pub const DCDEPTH_MAX: u32 = 1 << 5;
pub const ACDEPTH_MAX: u32 = 1 << 5;

pub const ENUM_NONE: u8 = 0;
pub const ENUM_TYPE_P: u8 = 1;
pub const ENUM_TRAN_B: u8 = 2;
pub const ENUM_TRAN_D: u8 = 3;
pub const ENUM_TYPE_CI: u8 = 4;
pub const ENUM_TRAN_GI: u8 = 5;
pub const ENUM_TRAN_HI: u8 = 6;
pub const ENUM_TYPE_HIJ: u8 = 7;

#[inline]
pub fn amplitude(a: i32) -> i32 {
    if a >= 0 {
        a
    } else {
        -a
    }
}

#[inline]
pub fn sign_of(var: i32) -> u8 {
    if var < 0 {
        NEGATIVE_SIGN
    } else {
        POSITIVE_SIGN
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolDetails {
    pub sym_val: u8,
    pub sym_len: u8,
    pub sym_mapped_pattern: u8,
    pub sign: u8,
    pub type_: u8,
}

#[derive(Debug, Clone, Default)]
pub struct TypeC {
    pub type_c: u8,
}

#[derive(Debug, Clone, Default)]
pub struct TranH {
    pub tran_h: u8,
}

#[derive(Debug, Clone, Default)]
pub struct TranHi {
    pub tran_h: u8,
}

#[derive(Debug, Clone, Default)]
pub struct TypeHij {
    pub type_hij: [TranHi; 4],
}

#[derive(Debug, Clone, Default)]
pub struct ParentRefine {
    pub parent_ref_symbol: u8,
    pub parent_symbol_length: u8,
}

#[derive(Debug, Clone, Default)]
pub struct ChildrenRefine {
    pub children_ref_symbol: u16,
    pub children_symbol_length: u8,
}

#[derive(Debug, Clone, Default)]
pub struct GrandChildrenRefine {
    pub grand_children_ref_symbol: u16,
    pub grand_children_symbol_length: u8,
}

#[derive(Debug, Clone, Default)]
pub struct RefineBits {
    pub refine_parent: ParentRefine,
    pub refine_children: ChildrenRefine,
    pub refine_grand_children: [GrandChildrenRefine; 3],
}

#[derive(Debug, Clone, Default)]
pub struct PlaneHit {
    pub type_p: u8,
    pub tran_b: u8,
    pub tran_d: u8,
    pub type_ci: [TypeC; 3],
    pub tran_gi: u8,
    pub tran_hi: [TranH; 3],
    pub type_hij: [TypeHij; 3],
}

#[derive(Debug, Clone)]
pub struct BitPlaneBits {
    /// Index into block_string rows: block_index * BLOCK_SIZE is the start row.
    pub block_index: i32,
    pub bit_plane: u8,
    pub mapped_dc: u32,
    pub shifted_dc: u32,
    pub dc_remainder: u16,
    pub decoding_dc_remainder: f32,
    pub bit_max_ac: u16,
    pub mapped_ac: u16,
    pub str_plane_hit_history: PlaneHit,
    pub symbols_block: [SymbolDetails; MAX_SYMBOLS_IN_BLOCK],
    pub refine_bits: RefineBits,
    /// Local 8x8 integer coefficients (copy / working set).
    pub block_int: [[i32; BLOCK_SIZE]; BLOCK_SIZE],
    pub block_float: [[f32; BLOCK_SIZE]; BLOCK_SIZE],
}

impl Default for BitPlaneBits {
    fn default() -> Self {
        Self {
            block_index: 0,
            bit_plane: 0,
            mapped_dc: 0,
            shifted_dc: 0,
            dc_remainder: 0,
            decoding_dc_remainder: 0.0,
            bit_max_ac: 0,
            mapped_ac: 0,
            str_plane_hit_history: PlaneHit::default(),
            symbols_block: std::array::from_fn(|_| SymbolDetails::default()),
            refine_bits: RefineBits::default(),
            block_int: [[0; BLOCK_SIZE]; BLOCK_SIZE],
            block_float: [[0.0; BLOCK_SIZE]; BLOCK_SIZE],
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderPart1 {
    pub start_img_flag: bool,
    pub eng_img_flg: bool,
    pub segment_count_8bits: u8,
    pub bit_depth_dc_5bits: u8,
    pub bit_depth_ac_5bits: u8,
    pub reserved: bool,
    pub part2_flag: bool,
    pub part3_flag: bool,
    pub part4_flag: bool,
    pub pad_rows_3bits: u8,
    pub reserved_5bits: u8,
}

impl Default for HeaderPart1 {
    fn default() -> Self {
        Self {
            start_img_flag: true,
            eng_img_flg: false,
            segment_count_8bits: 0,
            bit_depth_dc_5bits: 0,
            bit_depth_ac_5bits: 0,
            reserved: false,
            part2_flag: true,
            part3_flag: true,
            part4_flag: true,
            pad_rows_3bits: 0,
            reserved_5bits: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderPart2 {
    pub seg_byte_limit_27bits: u32,
    pub dc_stop: bool,
    pub bit_plane_stop_5bits: u8,
    pub stage_stop_2bits: u8,
    pub use_fill: bool,
    pub reserved_4bits: u8,
}

impl Default for HeaderPart2 {
    fn default() -> Self {
        Self {
            seg_byte_limit_27bits: 0,
            dc_stop: false,
            bit_plane_stop_5bits: 0,
            stage_stop_2bits: 3,
            use_fill: false,
            reserved_4bits: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderPart3 {
    pub s_20bits: u32,
    pub opt_dc_select: bool,
    pub opt_ac_select: bool,
    pub reserved_2bits: u8,
}

impl Default for HeaderPart3 {
    fn default() -> Self {
        Self {
            s_20bits: 256,
            opt_dc_select: true,
            opt_ac_select: true,
            reserved_2bits: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeaderPart4 {
    pub dwt_type: u8,
    pub reserved_2bits: u8,
    pub signed_pixels: bool,
    pub pixel_bit_depth_4bits: u8,
    pub image_width_20bits: u32,
    pub transpose_img: u8,
    pub codeword_length_2bits: u8,
    pub reserved: bool,
    pub custom_wt_flag: bool,
    pub custom_wt_hh1: u8,
    pub custom_wt_hl1: u8,
    pub custom_wt_lh1: u8,
    pub custom_wt_hh2: u8,
    pub custom_wt_hl2: u8,
    pub custom_wt_lh2: u8,
    pub custom_wt_hh3: u8,
    pub custom_wt_hl3: u8,
    pub custom_wt_lh3: u8,
    pub custom_wt_ll3: u8,
    pub reserved_11bits: u16,
}

impl Default for HeaderPart4 {
    fn default() -> Self {
        Self {
            dwt_type: INTEGER_WAVELET,
            reserved_2bits: 0,
            signed_pixels: false,
            pixel_bit_depth_4bits: 8,
            image_width_20bits: 2048,
            transpose_img: NOTRANSPOSE,
            codeword_length_2bits: 0,
            reserved: false,
            custom_wt_flag: false,
            custom_wt_hh1: 0,
            custom_wt_hl1: 1,
            custom_wt_lh1: 1,
            custom_wt_hh2: 1,
            custom_wt_hl2: 2,
            custom_wt_lh2: 2,
            custom_wt_hh3: 2,
            custom_wt_hl3: 3,
            custom_wt_lh3: 3,
            custom_wt_ll3: 3,
            reserved_11bits: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Header {
    pub part1: HeaderPart1,
    pub part2: HeaderPart2,
    pub part3: HeaderPart3,
    pub part4: HeaderPart4,
}

#[derive(Debug, Clone)]
pub struct StopLocation {
    pub bit_plane_stop_decoding: i8,
    pub block_no_stop_decoding: i32,
    pub total_bits_read_this_time: i16,
    pub location_find: bool,
    pub x_location_stop_decoding: i8,
    pub y_location_stop_decoding: i8,
    pub stopped_stage: u8,
}

impl Default for StopLocation {
    fn default() -> Self {
        Self {
            bit_plane_stop_decoding: -1,
            block_no_stop_decoding: -1,
            total_bits_read_this_time: 0,
            location_find: false,
            x_location_stop_decoding: -1,
            y_location_stop_decoding: -1,
            stopped_stage: 10,
        }
    }
}

#[derive(Debug)]
pub struct CodingPara {
    pub bits: BitStream,
    pub bit_plane: u8,
    pub bits_per_pixel: f32,
    pub decoding_allowed_bits_size_in_segment: u32,
    pub rate_reached: bool,
    pub decoding_stop_locations: StopLocation,
    pub quantization_factor_q: u8,
    pub block_counter: u32,
    pub block_index: u32,
    pub n: u8,
    pub segment_full: bool,
    pub header: Header,
    pub image_rows: u32,
    pub image_width: u32,
    pub pad_cols_3bits: u8,
    pub pixel_byte_order: u8,
    pub input_file: String,
    pub coding_output_file: String,
    /// When set, the float 9/7 inverse lifting reproduces the C reference's
    /// per-operator f32 rounding bit-for-bit instead of the more precise
    /// (but C-diverging) f64 accumulation. See `--compat-c-ref` in main.rs.
    pub strict_c_compat: bool,
}

impl CodingPara {
    pub fn new() -> Self {
        let mut header = Header::default();
        if header.part2.seg_byte_limit_27bits == 0 {
            header.part2.use_fill = false;
        } else {
            header.part2.use_fill = true;
        }
        let codeword_length = match header.part4.codeword_length_2bits {
            0 => 8u8,
            1 => 16,
            2 => 24,
            3 => 32,
            _ => 8,
        };
        Self {
            bits: BitStream::new(codeword_length),
            bit_plane: 0,
            bits_per_pixel: 0.0,
            decoding_allowed_bits_size_in_segment: 0,
            rate_reached: false,
            decoding_stop_locations: StopLocation::default(),
            quantization_factor_q: 0,
            block_counter: 0,
            block_index: 0,
            n: 0,
            segment_full: false,
            header,
            image_rows: 0,
            image_width: 0,
            pad_cols_3bits: 0,
            pixel_byte_order: 0,
            input_file: String::new(),
            coding_output_file: String::new(),
            strict_c_compat: false,
        }
    }
}

impl Default for CodingPara {
    fn default() -> Self {
        Self::new()
    }
}

/// Block string layout matching C: TotalBlocks * BLOCK_SIZE rows of BLOCK_SIZE longs.
/// Block b occupies rows [b*8 .. b*8+8), each of length 8.
pub type BlockString = Vec<[i32; BLOCK_SIZE]>;

pub fn alloc_block_string(total_blocks: usize) -> BlockString {
    vec![[0i32; BLOCK_SIZE]; total_blocks * BLOCK_SIZE]
}

/// Image as row-major Vec of rows.
pub type ImageI32 = Vec<Vec<i32>>;
pub type ImageF32 = Vec<Vec<f32>>;

pub fn alloc_image_i32(rows: usize, cols: usize) -> ImageI32 {
    vec![vec![0i32; cols]; rows]
}

pub fn alloc_image_f32(rows: usize, cols: usize) -> ImageF32 {
    vec![vec![0.0f32; cols]; rows]
}
