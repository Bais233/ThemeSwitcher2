fn main() {
    // exe 资源图标(仅 Windows 目标)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/app.ico");
        res.compile().expect("failed to embed app.ico");
    }
    slint_build::compile("ui/dashboard.slint").expect("failed to compile Slint UI");
}
