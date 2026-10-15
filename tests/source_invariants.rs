//! Static source invariants guarding REVIEW B7(d) and B8. They scan the
//! crate's own sources, so they run on any CPU.

use std::path::Path;

fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push((p.display().to_string(), std::fs::read_to_string(&p).unwrap()));
            }
        }
    }
    let mut v = Vec::new();
    walk(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut v);
    v
}

/// Body of every `fn` (brace-matched), with its name.
fn functions(src: &str) -> Vec<(String, &str)> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while let Some(off) = src[i..].find("fn ") {
        let start = i + off;
        i = start + 3;
        if start > 0 && (b[start - 1].is_ascii_alphanumeric() || b[start - 1] == b'_') {
            continue;
        }
        let name: String = src[start + 3..].chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let (Some(open), semi) = (src[start..].find('{'), src[start..].find(';')) else { continue };
        if semi.is_some_and(|s| s < open) {
            continue; // declaration without body
        }
        let open = start + open;
        let mut depth = 0;
        let mut end = open;
        for (k, c) in src[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + k;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((name, &src[open..=end]));
    }
    out
}

/// Every function that issues non-temporal stores must also issue `_mm_sfence`
/// (std::arch: "Safety of non-temporal stores").
#[test]
fn every_nt_store_is_fenced() {
    let mut missing = Vec::new();
    for (path, src) in sources() {
        for (name, body) in functions(&src) {
            let innermost = !body[1..].contains("fn ");
            if innermost && body.contains("_stream_s") && !body.contains("_mm_sfence") {
                missing.push(format!("{path}::{name}"));
            }
        }
    }
    assert!(missing.is_empty(), "NT stores without _mm_sfence: {missing:#?}");
}

/// AVX2-advertised code must not use AVX-512VL 256-bit intrinsics.
#[test]
fn no_avx512vl_256bit_intrinsics() {
    let banned = ["_mm256_rol_epi", "_mm256_ror_epi", "_mm256_mask_", "_mm256_maskz_", "_mm256_loadu_epi32", "_mm256_storeu_epi32", "_mm_rol_epi"];
    let mut hits = Vec::new();
    for (path, src) in sources() {
        for (n, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if banned.iter().any(|b| code.contains(b)) {
                hits.push(format!("{path}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(hits.is_empty(), "{hits:#?}");
}

/// C ABI functions that dereference a handle pointer must be `unsafe` for Rust
/// callers (REVIEW B7(b)).
#[test]
fn pointer_taking_c_abi_functions_are_unsafe() {
    let mut safe = Vec::new();
    for (path, src) in sources() {
        let flat = src.replace(['\r', '\n'], " ");
        let mut rest = flat.as_str();
        while let Some(i) = rest.find("extern \"C\" fn ") {
            let before = &rest[..i];
            let sig_end = rest[i..].find('{').map(|e| i + e).unwrap_or(rest.len());
            let sig = &rest[i..sig_end];
            if sig.contains("ptr: *mut") && !before.trim_end().ends_with("unsafe") {
                safe.push(format!("{path}: {}", sig.trim()));
            }
            rest = &rest[i + 14..];
        }
    }
    assert!(safe.is_empty(), "{safe:#?}");
}
