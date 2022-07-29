use std::path::Path;

fn main() -> anyhow::Result<()> {
    if cfg!(feature = "generate") {
        generator::generate(
            Path::new(&std::env::var("OUT_DIR")?).join("smart-home.rs"),
            Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join("smart-home-schema"),
        )
    } else {
        Ok(())
    }
}
