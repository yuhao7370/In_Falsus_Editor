#[cfg(windows)]
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("-windows-") {
        return;
    }

    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.compile().expect("failed to compile Windows resources");
}

#[cfg(not(windows))]
fn main() {}
