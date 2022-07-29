use std::path::Path;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next().unwrap();
    let schema_root = args.next().expect("First argument should be root of schema");
    let schema_root = Path::new(&schema_root);
    let out_path = args.next().expect("Second argument should be output path");
    let out_path = Path::new(&out_path);

    generator::generate(schema_root, out_path)
}
