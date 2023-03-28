use std::{env, error::Error, fs, path::Path};

pub fn compile_parser(parser_c: &Path, parser_h: &Path) -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    fs::copy(parser_c, dir.path().join("parser.c"))?;

    let header_dir = dir.path().join("tree_sitter");
    fs::create_dir(&header_dir)?;
    fs::copy(parser_h, header_dir.join("parser.h"))?;

    let sysroot_dir = dir.path().join("sysroot");
    if env::var("TARGET")?.starts_with("wasm32") {
        fs::create_dir(&sysroot_dir)?;

        fs::write(
            sysroot_dir.join("stdint.h"),
            include_bytes!("../wasm-sysroot/stdint.h"),
        )?;
        fs::write(
            sysroot_dir.join("stdlib.h"),
            include_bytes!("../wasm-sysroot/stdlib.h"),
        )?;
        fs::write(
            sysroot_dir.join("stdio.h"),
            include_bytes!("../wasm-sysroot/stdio.h"),
        )?;
        fs::write(
            sysroot_dir.join("stdbool.h"),
            include_bytes!("../wasm-sysroot/stdbool.h"),
        )?;
    }

    cc::Build::new()
        .include(&dir)
        .include(&sysroot_dir)
        .flag_if_supported("-Wno-unused-label")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-unknown-warning-option")
        .file(dir.path().join("parser.c"))
        .compile("parser");

    Ok(())
}
