#[cfg(feature = "slint-ui")]
fn main() {
    slint_build::compile("ui/main.slint").expect("Slint compilation failed");
}

#[cfg(not(feature = "slint-ui"))]
fn main() {}
