use std::{fs, path::PathBuf};

use ely_gpui_component::{
    Assets,
    theme::{ActiveTheme, Mode, Theme},
};
use gpui::{App, Application};
use luna_terminal::settings::{Settings, SettingsStore, interface_font, load};

pub fn run() {
    let root = std::env::temp_dir().join(format!("luna-terminal-ui-font-{}", std::process::id()));
    fs::create_dir_all(&root).expect("isolated home");
    // This test process has no threads or platform yet.
    unsafe { std::env::set_var("HOME", &root) };
    let path = root.join(".config/luna/settings.json");
    assert!(path.starts_with(&root), "never touch real settings");
    fs::create_dir_all(path.parent().unwrap()).expect("config folder");
    fs::write(
        &path,
        r#"{"font_family":"Menlo","ui_font_family":"Georgia"}"#,
    )
    .unwrap();
    Application::with_platform(gpui_platform::current_platform(true))
        .with_assets(Assets)
        .run(move |cx| {
            ely_gpui_component::init(cx).expect("Ely");
            startup_failures(&path, cx);
            SettingsStore::init(cx).expect("valid settings");
            let default = cx.theme().font_family.clone();
            let mono = cx.theme().mono_family.clone();
            let scale = cx.theme().font_scale;
            interface_font::init(cx);
            assert_eq!(cx.theme().font_family.as_ref(), "Georgia");
            cx.spawn(async move |cx| {
                cx.update(|_| ());
                cx.background_executor().timer(std::time::Duration::from_millis(10)).await;
                cx.update(|cx| {
                    for family in ["No Such Family", "Apple Color Emoji", " \t"] {
                        let before = fs::read(&path).unwrap();
                        let error = SettingsStore::update(cx, |s| s.ui_font_family = Some(family.into()))
                            .expect_err("invalid family refused");
                        assert!(format!("{error:#}").contains("ui_font_family"));
                        assert_eq!(SettingsStore::get(cx).ui_font_family.as_deref(), Some("Georgia"));
                        assert_eq!(fs::read(&path).unwrap(), before);
                    }
                    Theme::set_mode_now(Mode::Dark, cx);
                    Theme::set_palette(Mode::Light, None, cx);
                    assert_eq!(cx.theme().font_family.as_ref(), "Georgia");
                    SettingsStore::update(cx, |s| s.ui_font_family = Some("Times New Roman".into())).unwrap();
                });
                cx.background_executor().timer(std::time::Duration::from_millis(10)).await;
                cx.update(|cx| {
                    assert_eq!(cx.theme().font_family.as_ref(), "Times New Roman");
                    assert_eq!(load(&path).unwrap().ui_font_family.as_deref(), Some("Times New Roman"));
                    let saved = fs::read(&path).unwrap();
                    fs::remove_file(&path).unwrap();
                    fs::create_dir(&path).unwrap();
                    assert!(SettingsStore::update(cx, |s| s.ui_font_family = Some("Georgia".into())).is_err());
                    assert_eq!(SettingsStore::get(cx).ui_font_family.as_deref(), Some("Times New Roman"));
                    assert_eq!(cx.theme().font_family.as_ref(), "Times New Roman");
                    fs::remove_dir(&path).unwrap();
                    fs::write(&path, saved).unwrap();
                    SettingsStore::update(cx, |s| s.ui_font_family = None).unwrap();
                });
                cx.background_executor().timer(std::time::Duration::from_millis(10)).await;
                cx.update(|cx| {
                    assert_eq!(cx.theme().font_family, default);
                    assert_eq!(interface_font::family(cx), default);
                    assert_eq!(load(&path).unwrap().ui_font_family, None);
                    assert_eq!(cx.theme().mono_family, mono);
                    assert_eq!(cx.theme().font_scale, scale);
                    let settings = SettingsStore::get(cx);
                    assert_eq!(settings.font_family, "Menlo");
                    assert_eq!(settings.font_size, Settings::default().font_size);
                    assert!(settings.font_fallbacks.is_empty());
                    fs::remove_dir_all(&root).unwrap();
                    println!("interface fonts: startup, rollback, theme isolation, persistence and reset pass");
                    cx.quit();
                });
            }).detach();
        });
}

fn startup_failures(path: &PathBuf, cx: &mut App) {
    let valid = fs::read(path).unwrap();
    for family in ["No Such Family", "Apple Color Emoji"] {
        let text = format!(r#"{{"font_family":"Menlo","ui_font_family":"{family}"}}"#);
        fs::write(path, &text).unwrap();
        let error = SettingsStore::init(cx).expect_err("unloadable startup family");
        assert!(format!("{error:#}").contains("ui_font_family"));
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
    fs::write(path, valid).unwrap();
}
