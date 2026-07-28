//! Rice code-option selection per gaggle - original/source/PatternCoding.c

use crate::error::{BpeError, BpeResult};
use crate::types::{BitPlaneBits, CodingPara, ENUM_NONE, MAX_SYMBOLS_IN_BLOCK};

use super::mapping::pattern_mapping;

/// Important: like C, this mutates each symbol via pattern_mapping so that
/// subsequent Rice coding sees sym_mapped_pattern (BlockScan only sets sym_val).
pub fn coding_options(
    coding: &CodingPara,
    block_info: &mut [BitPlaneBits],
    blocks_in_gaggle: usize,
    option: &mut [u8; 3],
) -> BpeResult<()> {
    let mut bits_counter_2bits: [u32; 2] = [0; 2];
    let mut bits_counter_3bits: [u32; 3] = [0; 3];
    let mut bits_counter_4bits: [u32; 4] = [0; 4];

    for block_seq in 0..blocks_in_gaggle {
        if block_info[block_seq].bit_max_ac < coding.bit_plane as u16 {
            continue;
        }
        for symbol_index in 0..MAX_SYMBOLS_IN_BLOCK {
            if block_info[block_seq].symbols_block[symbol_index].type_ == ENUM_NONE {
                continue;
            }
            // C always PatternMapping before continue/statistics.
            pattern_mapping(&mut block_info[block_seq].symbols_block[symbol_index])?;
            let sym_len = block_info[block_seq].symbols_block[symbol_index].sym_len;
            if sym_len == 1 {
                continue;
            }
            let mapped = block_info[block_seq].symbols_block[symbol_index].sym_mapped_pattern;
            match sym_len {
                2 => {
                    match mapped {
                        0 => bits_counter_2bits[0] += 1,
                        1 => bits_counter_2bits[0] += 2,
                        2 => bits_counter_2bits[0] += 3,
                        3 => bits_counter_2bits[0] += 3,
                        _ => return Err(BpeError::PatternCodingError),
                    }
                    bits_counter_2bits[1] += 2;
                }
                3 => {
                    if mapped <= 2 {
                        bits_counter_3bits[0] += mapped as u32 + 1;
                    } else if mapped <= 5 {
                        bits_counter_3bits[0] += 5;
                    } else if mapped <= 7 {
                        bits_counter_3bits[0] += 6;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 1 {
                        bits_counter_3bits[1] += 2;
                    } else if mapped <= 3 {
                        bits_counter_3bits[1] += 3;
                    } else if mapped <= 7 {
                        bits_counter_3bits[1] += 4;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }
                    bits_counter_3bits[2] += 3;
                }
                4 => {
                    if mapped <= 3 {
                        bits_counter_4bits[0] += mapped as u32 + 1;
                    } else if mapped <= 7 {
                        bits_counter_4bits[0] += 7;
                    } else if mapped <= 15 {
                        bits_counter_4bits[0] += 8;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 1 {
                        bits_counter_4bits[1] += 2;
                    } else if mapped <= 3 {
                        bits_counter_4bits[1] += 3;
                    } else if mapped <= 5 {
                        bits_counter_4bits[1] += 4;
                    } else if mapped <= 11 {
                        bits_counter_4bits[1] += 6;
                    } else if mapped <= 15 {
                        bits_counter_4bits[1] += 7;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    if mapped <= 3 {
                        bits_counter_4bits[2] += 3;
                    } else if mapped <= 7 {
                        bits_counter_4bits[2] += 4;
                    } else if mapped <= 15 {
                        bits_counter_4bits[2] += 5;
                    } else {
                        return Err(BpeError::PatternCodingError);
                    }

                    bits_counter_4bits[3] += 4;
                }
                _ => {}
            }
        }
    }

    if bits_counter_2bits[0] < bits_counter_2bits[1] {
        option[0] = 0;
    } else {
        option[0] = 1;
    }

    if bits_counter_3bits[2] <= bits_counter_3bits[0]
        && bits_counter_3bits[2] <= bits_counter_3bits[1]
    {
        option[1] = 3;
    } else if bits_counter_3bits[0] <= bits_counter_3bits[1]
        && bits_counter_3bits[0] <= bits_counter_3bits[2]
    {
        option[1] = 0;
    } else if bits_counter_3bits[1] <= bits_counter_3bits[0]
        && bits_counter_3bits[1] <= bits_counter_3bits[2]
    {
        option[1] = 1;
    }

    if bits_counter_4bits[3] <= bits_counter_4bits[1]
        && bits_counter_4bits[3] <= bits_counter_4bits[0]
        && bits_counter_4bits[3] <= bits_counter_4bits[2]
    {
        option[2] = 3;
    } else if bits_counter_4bits[0] <= bits_counter_4bits[1]
        && bits_counter_4bits[0] <= bits_counter_4bits[2]
        && bits_counter_4bits[0] <= bits_counter_4bits[3]
    {
        option[2] = 0;
    } else if bits_counter_4bits[1] <= bits_counter_4bits[0]
        && bits_counter_4bits[1] <= bits_counter_4bits[2]
        && bits_counter_4bits[1] <= bits_counter_4bits[3]
    {
        option[2] = 1;
    } else if bits_counter_4bits[2] <= bits_counter_4bits[1]
        && bits_counter_4bits[2] <= bits_counter_4bits[0]
        && bits_counter_4bits[2] <= bits_counter_4bits[3]
    {
        option[2] = 2;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Cross-checks against verify/vectors/codingoptions_vectors.txt, generated
    /// by verify/c_unit_tests/gen_codingoptions_vectors.c, which calls the real
    /// C `CodingOptions` directly (not through a full encode/decode roundtrip)
    /// across single-symbol, pairwise, and exhaustive-triple sweeps of every
    /// sym_len/type combination, plus two hand-derived cases that reach
    /// tie-break directions no generic sweep hits (see that generator's and
    /// COMPATIBILITY_REPORT.md §4 item 8's comments for why those two exist).
    ///
    /// Ignored by default (like the other shared-vector tests) because it
    /// needs verify/run_unit_vectors.py to have generated the vectors file
    /// first; that script runs this test with `--include-ignored`.
    #[test]
    #[ignore]
    fn shared_vectors_match_c_reference() {
        let vectors_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../verify/vectors/codingoptions_vectors.txt");
        let text = fs::read_to_string(&vectors_path).unwrap_or_else(|e| {
            panic!(
                "couldn't read {}: {e} (run verify/run_unit_vectors.py first)",
                vectors_path.display()
            )
        });

        let mut checked = 0;
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let [sym_len, type_, n, sym_val_csv, o0, o1, o2] = fields[..] else {
                panic!("malformed vector line: {line}");
            };
            let sym_len: u8 = sym_len.parse().unwrap();
            let type_: u8 = type_.parse().unwrap();
            let n: usize = n.parse().unwrap();
            let sym_vals: Vec<u8> = sym_val_csv.split(',').map(|v| v.parse().unwrap()).collect();
            assert_eq!(sym_vals.len(), n);
            let expected: [u8; 3] = [
                o0.parse().unwrap(),
                o1.parse().unwrap(),
                o2.parse().unwrap(),
            ];

            let mut coding = CodingPara::new();
            coding.bit_plane = 1;

            let mut block = BitPlaneBits {
                bit_max_ac: 20,
                ..Default::default()
            };
            for (i, &v) in sym_vals.iter().enumerate() {
                block.symbols_block[i].type_ = type_;
                block.symbols_block[i].sym_len = sym_len;
                block.symbols_block[i].sym_val = v;
            }
            let mut blocks = [block];

            let mut got = [0u8; 3];
            coding_options(&coding, &mut blocks, 1, &mut got).unwrap();

            assert_eq!(
                got, expected,
                "sym_len={} type={} sym_vals={:?}: option mismatch: rust={:?} c={:?}",
                sym_len, type_, sym_vals, got, expected
            );

            checked += 1;
        }
        assert!(checked > 0, "vectors file was empty");
    }
}
