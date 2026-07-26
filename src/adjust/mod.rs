//! Rate-budget output adjust — original/source/AdjustOutput.c
//!
//! Layout: `common` (bump / refine), one file per `stopped_stage`
//! (`stage1`..`stage4`), and `orchestrate` (`adjust_output` / `dispatch_stage`).
//!
//! `AdjustOutPut()` runs only on the decoder side, after decoding has stopped
//! early because the rate budget was exhausted. It nudges every coefficient
//! that was not fully refined towards the midpoint of the uncertainty
//! interval implied by the bit-plane / stage / (block, x, y) location where
//! decoding stopped.
//!
//! In the original C, the whole function is duplicated almost verbatim once
//! for `INTEGER_WAVELET` (updating `PtrBlockAddress` as the primary integer
//! array, with `PtrBlockAddressFloating` kept in lock-step) and once more for
//! the floating wavelet case (same branching, `beta_1`/`beta_2`/`BitPlaneCheck`
//! computed with a `- 0.5` bias instead of `- 1`). Line-by-line comparison of
//! the two halves (identical branch conditions, identical comments, only the
//! numeric literal types differ) confirms the branching logic itself is
//! shared, so it is implemented once here as `stage1`..`stage4` and invoked
//! with the appropriate `(beta_1, beta_2, BitPlaneCheck)` triple for either
//! wavelet type. Both the integer and the floating coefficient copies
//! (`block_int` / `block_float`) are always updated together by `bump()`,
//! exactly mirroring the original which updates `PtrBlockAddress` and
//! `PtrBlockAddressFloating` side by side in every branch.

mod common;
mod orchestrate;
mod stage1;
mod stage2;
mod stage3;
mod stage4;

pub use orchestrate::adjust_output;
