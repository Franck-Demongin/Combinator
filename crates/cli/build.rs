fn main() {
    #[cfg(all(windows, feature = "slint-ui"))]
    {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set");
        let icon_path =
            std::path::Path::new(&manifest_dir).join("../../assets/icons/combinator.ico");
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon(
            icon_path
                .to_str()
                .expect("Windows icon path should be valid UTF-8"),
        );
        resource.set("FileDescription", "Composition automatique de planches PDF");
        resource.set("ProductName", "Combinator");
        resource
            .compile()
            .expect("failed to compile Windows resources");
    }

    #[cfg(feature = "slint-ui")]
    slint_build::compile("ui/main.slint").expect("Slint compilation failed");
}
