use std::fs;
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let output = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/cli.md");
    fs::create_dir_all(
        output
            .parent()
            .expect("CLI documentation has a parent directory"),
    )?;
    fs::write(&output, wavlab::cli::reference_markdown())?;
    println!("generated {}", output.display());
    Ok(())
}
