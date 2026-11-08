// tray.rs — 托盘图标与右键菜单
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct MenuHandles {
    pub status: MenuItem,
    pub to_light: MenuItem,
    pub to_dark: MenuItem,
    pub autostart: CheckMenuItem,
    pub open_config: MenuItem,
    pub reload: MenuItem,
    pub quit: MenuItem,
}

fn build_icon() -> Option<Icon> {
    let png = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Icon::from_rgba(rgba.into_raw(), w, h).ok()
}

pub fn build() -> (TrayIcon, MenuHandles) {
    let status = MenuItem::new("启动中…", false, None);
    let to_light = MenuItem::new("切换到浅色模式", true, None);
    let to_dark = MenuItem::new("切换到深色模式", true, None);
    let autostart = CheckMenuItem::new("开机自启", true, false, None);
    let open_config = MenuItem::new("打开配置文件", true, None);
    let reload = MenuItem::new("重读配置", true, None);
    let quit = MenuItem::new("退出", true, None);

    let mut menu = Menu::new();
    let _ = menu.append_items(&[
        &status,
        &PredefinedMenuItem::separator(),
        &to_light,
        &to_dark,
        &PredefinedMenuItem::separator(),
        &autostart,
        &open_config,
        &reload,
        &PredefinedMenuItem::separator(),
        &quit,
    ]);

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("ThemeSwitcher2");
    if let Some(icon) = build_icon() {
        builder = builder.with_icon(icon);
    }
    let tray = builder.build().expect("failed to create tray icon");
    (
        tray,
        MenuHandles {
            status,
            to_light,
            to_dark,
            autostart,
            open_config,
            reload,
            quit,
        },
    )
}
