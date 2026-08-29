use std::path::{Path, PathBuf};

const MAX_SOURCE_LINES: usize = 300;

fn rust_sources_in(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("listar fontes Rust") {
        let entry = entry.expect("ler entrada de fonte Rust");
        let path = entry.path();
        if path.is_dir() {
            rust_sources_in(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn rust_source_files_do_not_exceed_300_lines() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source_root = manifest_dir.join("src");
    let mut files = Vec::new();
    rust_sources_in(&source_root, &mut files);

    let violations: Vec<String> = files
        .into_iter()
        .filter_map(|path| {
            let lines = std::fs::read_to_string(&path)
                .expect("ler fonte Rust")
                .lines()
                .count();
            (lines > MAX_SOURCE_LINES).then(|| {
                format!(
                    "{}: {lines} linhas (máximo: {MAX_SOURCE_LINES})",
                    path.strip_prefix(manifest_dir)
                        .expect("fonte dentro do crate")
                        .display()
                )
            })
        })
        .collect();

    assert!(
        violations.is_empty(),
        "Arquivos Rust acima de {MAX_SOURCE_LINES} linhas:\n{}",
        violations.join("\n")
    );
}
