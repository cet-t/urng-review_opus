//! Regression test for REVIEW B1: the crate-level doc include must name a file
//! that exists with the *exact* same case, otherwise the crate fails to build on
//! case-sensitive file systems (Linux, docs.rs).

#[test]
fn lib_doc_include_path_matches_file_case_exactly() {
    let root = env!("CARGO_MANIFEST_DIR");
    let lib = std::fs::read_to_string(format!("{root}/src/lib.rs")).unwrap();
    let marker = "include_str!(\"../";
    let start = lib.find(marker).expect("lib.rs includes a root file") + marker.len();
    let len = lib[start..].find('"').unwrap();
    let name = &lib[start..start + len];
    let entries: Vec<String> = std::fs::read_dir(root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.iter().any(|e| e == name),
        "include_str!(\"../{name}\") but the crate root contains {entries:?}"
    );
}
