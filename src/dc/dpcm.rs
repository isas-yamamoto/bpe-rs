//! DPCM mapping/demapping for DC coefficients - original/source/DC_EnDeCoding.c

use crate::types::BitPlaneBits;

/// C: `-(short)(inner)`. The inner expression is computed in (at least)
/// 32-bit arithmetic on both sides, but C narrows it through a 16-bit
/// `short` *before* negating, while a direct `-(inner as i32)` would not.
/// For most magnitudes the narrowing is a no-op, but at inner == 32768 (the
/// case N == 16 hits whenever a block's raw value has its top bit set) the
/// short-cast wraps to -32768 first, so negating gives +32768 -- the
/// opposite sign from what negating the un-narrowed i32 would give.
///
/// This isn't a rounding nuance like the float-DWT precision (see
/// `strict_c_compat` in `lifting97f.rs`): the mapped value this quirk
/// produces gets Rice-coded straight into the bitstream, so unlike the
/// float lifting it isn't safe to always take the "clean" answer. Only
/// take it with `strict_c_compat = false` (`--fix-c-quirks`), where
/// interop with the C reference is explicitly not a goal; the default
/// `strict_c_compat = true` replicates the narrowing exactly.
#[inline]
fn neg_short(inner: i32, strict_c_compat: bool) -> i32 {
    if strict_c_compat {
        -((inner as i16) as i32)
    } else {
        -inner
    }
}

/// C: `theta = min(ShiftedDC - X_Min, X_Max - ShiftedDC)`. `ShiftedDC` and
/// `X_Max` are `DWORD32` (unsigned) while `X_Min` is `long` (signed); the
/// usual arithmetic conversions make both subtractions happen in unsigned
/// arithmetic. Ordinarily `ShiftedDC` stays within `[X_Min, X_Max]` and
/// this is equivalent to signed subtraction -- but `neg_short`'s overflow
/// (above) can push it outside that range, and when it does, one of the
/// two subtractions wraps around to a huge unsigned value instead of going
/// negative. C's unsigned `min()` then picks based on that huge wrapped
/// value, not the mathematically-intended negative one.
///
/// Same tradeoff as `neg_short`: `strict_c_compat = true` reproduces C's
/// unsigned wraparound (`wrapping_sub`) bit for bit; `strict_c_compat =
/// false` computes the mathematically-intended signed `min` instead, which
/// only an encoder/decoder pair that both skip the bug (not the real C
/// reference) can interoperate with.
#[inline]
fn theta_from_prev(prev: u32, x_min: i32, x_max: i32, strict_c_compat: bool) -> i32 {
    if strict_c_compat {
        let term1 = prev.wrapping_sub(x_min as u32);
        let term2 = (x_max as u32).wrapping_sub(prev);
        term1.min(term2) as i32
    } else {
        let prev = prev as i32;
        (prev - x_min).min(x_max - prev)
    }
}

pub fn dpcm_dc_mapper(block_info: &mut [BitPlaneBits], size: usize, n: i16, strict_c_compat: bool) {
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
        block_info[0].shifted_dc = neg_short(((sd0 ^ bits1) & bits1) + 1, strict_c_compat) as u32;
    }
    diff_dc[0] = block_info[0].shifted_dc as i32;

    for i in 1..size {
        let sdi = block_info[i].shifted_dc as i32;
        if (sdi & (1 << (n - 1))) > 0 {
            block_info[i].shifted_dc =
                neg_short(((sdi ^ bits1) & bits1) + 1, strict_c_compat) as u32;
        }
        diff_dc[i] = (block_info[i].shifted_dc as i32) - (block_info[i - 1].shifted_dc as i32);
    }

    for i in 1..size {
        let theta = theta_from_prev(block_info[i - 1].shifted_dc, x_min, x_max, strict_c_compat);
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

pub fn dpcm_dc_demapper(
    block_info: &mut [BitPlaneBits],
    size: usize,
    n: i16,
    strict_c_compat: bool,
) {
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
        block_info[0].shifted_dc = neg_short(((sd0 ^ bits1) & bits1) + 1, strict_c_compat) as u32;
    }

    for i in 1..size {
        let prev = block_info[i - 1].shifted_dc as i32;
        let theta = theta_from_prev(block_info[i - 1].shifted_dc, x_min, x_max, strict_c_compat);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    const N: i16 = 8;

    fn make_blocks(raw_dc: &[u32]) -> Vec<BitPlaneBits> {
        raw_dc
            .iter()
            .map(|&dc| BitPlaneBits {
                shifted_dc: dc,
                ..Default::default()
            })
            .collect()
    }

    /// Cross-checks against verify/vectors/dpcm_vectors.txt, generated by
    /// verify/c_unit_tests/gen_dpcm_vectors.c from the real C reference
    /// (linked against its actual DC_EnDeCoding.o). Each line is
    /// `N size raw_csv mapped_csv decoded_csv`: running dpcm_dc_mapper on
    /// raw_csv must produce mapped_csv, and running dpcm_dc_demapper on
    /// mapped_csv must produce decoded_csv, bit-for-bit against what the C
    /// DPCM_DCMapper/DPCM_DCDeMapper produced for the same input -- notably
    /// including the theta-boundary branch DPCM_DCDeMapper had rewritten as
    /// part of the Kiely bugfix (see readme_kielymods.rtf in the parent
    /// repo's original/ directory).
    ///
    /// Ignored by default (like golden_roundtrip.rs) because it needs
    /// verify/run_unit_vectors.py to have generated the vectors file first;
    /// that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../verify/vectors/dpcm_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let parse_csv =
            |s: &str| -> Vec<u32> { s.split(',').map(|v| v.parse().unwrap()).collect() };

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [n, size, raw_csv, mapped_csv, decoded_csv] = fields[..] else {
                panic!("malformed vector line: {line}");
            };
            let n: i16 = n.parse().unwrap();
            let size: usize = size.parse().unwrap();
            let raw = parse_csv(raw_csv);
            let expected_mapped = parse_csv(mapped_csv);
            let expected_decoded = parse_csv(decoded_csv);
            assert_eq!(raw.len(), size);

            let mut encoded = make_blocks(&raw);
            dpcm_dc_mapper(&mut encoded, size, n, true);
            let got_mapped: Vec<u32> = encoded.iter().map(|b| b.mapped_dc).collect();
            assert_eq!(
                got_mapped, expected_mapped,
                "N={} mapper mismatch: rust={:?} c={:?}",
                n, got_mapped, expected_mapped
            );

            let mut decoded = make_blocks(&vec![0; size]);
            for i in 0..size {
                decoded[i].mapped_dc = encoded[i].mapped_dc;
            }
            dpcm_dc_demapper(&mut decoded, size, n, true);
            let got_decoded: Vec<u32> = decoded.iter().map(|b| b.shifted_dc).collect();
            assert_eq!(
                got_decoded, expected_decoded,
                "N={} demapper mismatch: rust={:?} c={:?}",
                n, got_decoded, expected_decoded
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }

    #[test]
    fn mapper_then_demapper_restores_shifted_dc() {
        for strict_c_compat in [true, false] {
            let raw_dc: Vec<u32> = vec![0, 1, 2, 255, 128, 127, 64, 200, 3, 250];
            let size = raw_dc.len();

            let mut encoded = make_blocks(&raw_dc);
            dpcm_dc_mapper(&mut encoded, size, N, strict_c_compat);

            let mut decoded = make_blocks(&vec![0; size]);
            for i in 0..size {
                decoded[i].mapped_dc = encoded[i].mapped_dc;
            }
            dpcm_dc_demapper(&mut decoded, size, N, strict_c_compat);

            for i in 0..size {
                assert_eq!(
                    decoded[i].shifted_dc, encoded[i].shifted_dc,
                    "strict_c_compat={} block {} mismatch",
                    strict_c_compat, i
                );
            }
        }
    }

    /// `neg_short`'s overflow only fires when `shifted_dc`'s top bit (bit
    /// `n-1`) is set on a value whose narrowed negation should wrap -- at
    /// `N=16` that's `inner == 32768`. `strict_c_compat=true` must reproduce
    /// the C reference's wrap-to-`+32768`; `strict_c_compat=false` must give
    /// the mathematically-intended `-32768` (as an unsigned `u32` bit
    /// pattern, i.e. `(-32768i32) as u32`).
    #[test]
    fn neg_short_overflow_only_reproduced_in_strict_mode() {
        let raw_dc: Vec<u32> = vec![0, 1u32 << 15];
        let size = raw_dc.len();

        let mut strict = make_blocks(&raw_dc);
        dpcm_dc_mapper(&mut strict, size, 16, true);
        assert_eq!(strict[1].shifted_dc, 1u32 << 15);

        let mut clean = make_blocks(&raw_dc);
        dpcm_dc_mapper(&mut clean, size, 16, false);
        assert_eq!(clean[1].shifted_dc, (-(1i32 << 15)) as u32);
    }

    #[test]
    fn first_block_is_sent_verbatim() {
        let mut blocks = make_blocks(&[42, 43]);
        dpcm_dc_mapper(&mut blocks, 2, N, true);
        assert_eq!(blocks[0].mapped_dc, 42);
    }

    #[test]
    fn constant_dc_maps_to_zero_differences() {
        let mut blocks = make_blocks(&vec![100; 5]);
        dpcm_dc_mapper(&mut blocks, 5, N, true);
        for block in blocks.iter().skip(1) {
            assert_eq!(block.mapped_dc, 0);
        }
    }
}
