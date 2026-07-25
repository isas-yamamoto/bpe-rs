use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

#[test]
#[ignore]
fn rust_matches_c_encode_bytes() {
    let root = repo_root();
    let testdata = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let raw = testdata.join("test256.raw");
    assert!(raw.exists());
    let rust_bpe = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/bpe.exe");
    let c_bpe = root.join("original/source/bpe.exe");
    assert!(rust_bpe.exists() && c_bpe.exists());
    let out_r = testdata.join("gt_rust.bpe");
    let out_c = testdata.join("gt_c.bpe");
    let args_common = ["-r", "0", "-w", "256", "-h", "256", "-s", "256", "-t", "1", "-b", "8"];
    assert!(Command::new(&rust_bpe).args(["-e", raw.to_str().unwrap(), "-o", out_r.to_str().unwrap()]).args(args_common).status().unwrap().success());
    assert!(Command::new(&c_bpe).args(["-e", raw.to_str().unwrap(), "-o", out_c.to_str().unwrap()]).args(args_common).status().unwrap().success());
    assert_eq!(std::fs::read(&out_r).unwrap(), std::fs::read(&out_c).unwrap());
}
