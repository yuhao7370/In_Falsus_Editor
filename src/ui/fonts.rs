use egui_macroquad::egui::{self, FontFamily};
use macroquad::text::{Font, load_ttf_font};
use std::{path::Path, sync::Arc};

const PROJECT_FONT_CANDIDATES: &[&str] = &[
    "assets/HarmonyOS_Sans_SC_Regular.ttf",
    "assets/HarmonyOS_Sans_Regular.ttf",
    "assets/simhei.ttf",
    "assets/simhei.otf",
    "E:/RotaenoChartTool_rs/assets/HarmonyOS_Sans_SC_Regular.ttf",
    "E:/RotaenoChartTool_rs/assets/HarmonyOS_Sans_Regular.ttf",
];

#[cfg(target_os = "windows")]
const SYSTEM_FONT_CANDIDATES: &[&str] = &[
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/simsun.ttc",
    "C:/Windows/Fonts/simhei.ttf",
    "C:/Windows/Fonts/simkai.ttf",
    "C:/Windows/Fonts/msyhbd.ttc",
];

#[cfg(target_os = "linux")]
const SYSTEM_FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    "/usr/share/fonts/TTF/WenQuanYi-Micro-Hei.ttf",
];

#[cfg(target_os = "macos")]
const SYSTEM_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Supplemental/PingFang.ttc",
    "/System/Library/Fonts/STHeiti Light.ttc",
    "/Library/Fonts/Microsoft/Microsoft YaHei.ttf",
];

fn try_load_font(fonts: &mut egui::FontDefinitions, font_path: &str, font_name: &str) -> bool {
    let Ok(font_data) = std::fs::read(font_path) else {
        return false;
    };

    fonts.font_data.insert(
        font_name.to_owned(),
        Arc::new(egui::FontData::from_owned(font_data)),
    );
    true
}

fn register_font_family(fonts: &mut egui::FontDefinitions, font_name: String) {
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    if !proportional.contains(&font_name) {
        proportional.insert(0, font_name.clone());
    }

    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    if !monospace.contains(&font_name) {
        monospace.insert(0, font_name);
    }
}

fn pick_font_name(font_path: &str) -> String {
    Path::new(font_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("CjkFallback")
        .to_owned()
}

pub fn init_egui_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded_font_name = None;

    for font_path in PROJECT_FONT_CANDIDATES {
        let name = pick_font_name(font_path);
        if try_load_font(&mut fonts, font_path, &name) {
            loaded_font_name = Some(name);
            break;
        }
    }

    if loaded_font_name.is_none() {
        for font_path in SYSTEM_FONT_CANDIDATES {
            let name = pick_font_name(font_path);
            if try_load_font(&mut fonts, font_path, &name) {
                loaded_font_name = Some(name);
                break;
            }
        }
    }

    if let Some(font_name) = loaded_font_name {
        register_font_family(&mut fonts, font_name);
        ctx.set_fonts(fonts);
        return true;
    }

    false
}

pub async fn load_macroquad_cjk_font() -> Option<Font> {
    for font_path in PROJECT_FONT_CANDIDATES {
        if let Ok(font) = load_ttf_font(font_path).await {
            return Some(font);
        }
    }

    for font_path in SYSTEM_FONT_CANDIDATES {
        if let Ok(font) = load_ttf_font(font_path).await {
            return Some(font);
        }
    }

    None
}
