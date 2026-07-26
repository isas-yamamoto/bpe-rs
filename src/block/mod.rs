//! Block bit-plane scanning — original/source/BPEBlockCoding.c
//!
//! Layout: `common` (ScanCtx / symbol helpers), `orchestrate` (`block_scan_encode`),
//! one file per scan symbol family (`type_p`, `tran_b`, `tran_d`, `type_ci`,
//! `tran_gi`, `tran_hi`, `type_hij`).

mod common;
mod orchestrate;
mod tran_b;
mod tran_d;
mod tran_gi;
mod tran_hi;
mod type_ci;
mod type_hij;
mod type_p;

pub use orchestrate::block_scan_encode;
