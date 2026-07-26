//! Pixel byte-order helpers shared by the read and write paths.

/// The C source compares `PixelByteOrder` against the host endianness; this
/// port targets little-endian hosts, where that constant is 0.
const MACHINE_ENDIANNESS: u8 = 0;

/// Kiely endian fix: true when the image byte order differs from the host.
pub(crate) fn byte_order_differs(pixel_byte_order: u8) -> bool {
    pixel_byte_order != MACHINE_ENDIANNESS
}

/// Swap the two bytes of a 16-bit sample.
pub(crate) fn byte_swap_16(v: i32) -> i32 {
    ((v << 8) & 0xFF00) + (v >> 8)
}

/// Swap only when `swap` is set.
pub(crate) fn maybe_byte_swap(v: i32, swap: bool) -> i32 {
    if swap {
        byte_swap_16(v)
    } else {
        v
    }
}
