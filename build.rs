fn main() {
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![
            "ui".into(),
            "ui/components".into(),
        ]);
    slint_build::compile_with_config("ui/app.slint", config)
    .expect("Gagal mengompilasi UI Slint. Periksa sintaks di ui/app.slint atau path file.");
}
