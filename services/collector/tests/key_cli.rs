use std::process::Command;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

#[test]
fn key_generate_writes_only_one_key_to_stdout() {
    let output = Command::new(env!("CARGO_BIN_EXE_collector"))
        .args([
            "key",
            "generate",
            "--site",
            "site_example",
            "--environment",
            "production",
        ])
        .output()
        .expect("collector key command should start");

    assert!(output.status.success());
    assert_eq!(
        output.stdout.iter().filter(|byte| **byte == b'\n').count(),
        1
    );
    assert!(!output.stdout.contains(&b'\r'));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be UTF-8");
    let key = stdout
        .strip_suffix('\n')
        .expect("key should end with one newline");
    assert_eq!(key.len(), 43);
    assert_eq!(URL_SAFE_NO_PAD.decode(key).unwrap().len(), 32);
    assert!(!key.contains('='));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(key));
}
