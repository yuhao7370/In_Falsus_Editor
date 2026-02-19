pub mod app;
pub mod audio;
pub mod chart;
pub mod editor;
pub mod i18n;
pub mod settings;
pub mod ui;

use app::constants::*;
use app::input_handler;
use app::menu_actions::handle_top_menu_action;
use app::project_manager::ProjectManager;
use app::setup::apply_settings_to_editor;
#[cfg(target_os = "android")]
use app::setup::window_conf;
use app::ui_orchestrator::UiOrchestrator;
use audio::controller::AudioController;
use editor::falling::{FallingEditorAction, FallingGroundEditor};
use i18n::I18n;
use macroquad::prelude::*;
use settings::settings;
#[cfg(target_os = "android")]
use std::ffi::CStr;
#[cfg(target_os = "android")]
use std::sync::Once;
use ui::fonts::load_macroquad_cjk_font;
use ui::info_toast::InfoToastManager;
#[cfg(target_os = "android")]
use ui::input_state::android_play_pause_pressed;
use ui::input_state::update_touch_emulation;
use ui::progress_bar::{TopProgressBarState, draw_top_progress_bar};
use ui::scale::refresh_ui_scale;

#[cfg(target_os = "android")]
#[allow(dead_code)]
#[macroquad::main(window_conf)]
async fn main() {
    run_app().await;
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn quad_main() {
    main();
}

#[cfg(target_os = "android")]
fn android_cwd_for_error() -> String {
    std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "<unknown>".to_owned())
}

#[cfg(target_os = "android")]
unsafe fn set_android_working_dir_to_files_dir(
    env: *mut macroquad::miniquad::native::android::ndk_sys::JNIEnv,
    activity: macroquad::miniquad::native::android::ndk_sys::jobject,
) -> Result<(), String> {
    if env.is_null() || activity.is_null() {
        return Err("jni env/activity is null".to_owned());
    }

    let get_object_class = unsafe { (**env).GetObjectClass }
        .ok_or_else(|| "jni GetObjectClass is null".to_owned())?;
    let get_method_id = unsafe { (**env).GetMethodID }
        .ok_or_else(|| "jni GetMethodID is null".to_owned())?;
    let call_object_method = unsafe { (**env).CallObjectMethod }
        .ok_or_else(|| "jni CallObjectMethod is null".to_owned())?;
    let get_string_utf_chars = unsafe { (**env).GetStringUTFChars }
        .ok_or_else(|| "jni GetStringUTFChars is null".to_owned())?;
    let release_string_utf_chars = unsafe { (**env).ReleaseStringUTFChars }
        .ok_or_else(|| "jni ReleaseStringUTFChars is null".to_owned())?;
    let delete_local_ref = unsafe { (**env).DeleteLocalRef };

    let activity_class = unsafe { get_object_class(env, activity) };
    if activity_class.is_null() {
        return Err("failed to get Activity class".to_owned());
    }

    let files_dir_mid = unsafe {
        get_method_id(
            env,
            activity_class,
            b"getFilesDir\0".as_ptr() as *const _,
            b"()Ljava/io/File;\0".as_ptr() as *const _,
        )
    };
    if files_dir_mid.is_null() {
        if let Some(delete_local_ref) = delete_local_ref {
            unsafe { delete_local_ref(env, activity_class as _) };
        }
        return Err("failed to resolve Activity.getFilesDir()".to_owned());
    }

    let files_dir_obj = unsafe { call_object_method(env, activity, files_dir_mid) };
    if let Some(delete_local_ref) = delete_local_ref {
        unsafe { delete_local_ref(env, activity_class as _) };
    }
    if files_dir_obj.is_null() {
        return Err("Activity.getFilesDir() returned null".to_owned());
    }

    let file_class = unsafe { get_object_class(env, files_dir_obj) };
    if file_class.is_null() {
        if let Some(delete_local_ref) = delete_local_ref {
            unsafe { delete_local_ref(env, files_dir_obj as _) };
        }
        return Err("failed to get File class".to_owned());
    }

    let abs_path_mid = unsafe {
        get_method_id(
            env,
            file_class,
            b"getAbsolutePath\0".as_ptr() as *const _,
            b"()Ljava/lang/String;\0".as_ptr() as *const _,
        )
    };
    if abs_path_mid.is_null() {
        if let Some(delete_local_ref) = delete_local_ref {
            unsafe {
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, files_dir_obj as _);
            }
        }
        return Err("failed to resolve File.getAbsolutePath()".to_owned());
    }

    let files_dir_jstr =
        unsafe { call_object_method(env, files_dir_obj, abs_path_mid) as macroquad::miniquad::native::android::ndk_sys::jstring };
    if files_dir_jstr.is_null() {
        if let Some(delete_local_ref) = delete_local_ref {
            unsafe {
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, files_dir_obj as _);
            }
        }
        return Err("File.getAbsolutePath() returned null".to_owned());
    }

    let c_path = unsafe { get_string_utf_chars(env, files_dir_jstr, std::ptr::null_mut()) };
    if c_path.is_null() {
        if let Some(delete_local_ref) = delete_local_ref {
            unsafe {
                delete_local_ref(env, files_dir_jstr as _);
                delete_local_ref(env, file_class as _);
                delete_local_ref(env, files_dir_obj as _);
            }
        }
        return Err("GetStringUTFChars returned null".to_owned());
    }

    let path = unsafe { CStr::from_ptr(c_path) }
        .to_string_lossy()
        .to_string();
    unsafe { release_string_utf_chars(env, files_dir_jstr, c_path) };

    if let Some(delete_local_ref) = delete_local_ref {
        unsafe {
            delete_local_ref(env, files_dir_jstr as _);
            delete_local_ref(env, file_class as _);
            delete_local_ref(env, files_dir_obj as _);
        }
    }

    std::env::set_current_dir(&path).map_err(|err| format!("set_current_dir('{path}') failed: {err}"))
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_quad_1native_QuadNative_initAndroidContext(
    env: *mut macroquad::miniquad::native::android::ndk_sys::JNIEnv,
    _: macroquad::miniquad::native::android::ndk_sys::jclass,
    activity: macroquad::miniquad::native::android::ndk_sys::jobject,
) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        if env.is_null() || activity.is_null() {
            return;
        }

        let get_java_vm = unsafe { (**env).GetJavaVM };
        let new_global_ref = unsafe { (**env).NewGlobalRef };
        let Some(get_java_vm) = get_java_vm else {
            return;
        };
        let Some(new_global_ref) = new_global_ref else {
            return;
        };

        let mut vm: *mut macroquad::miniquad::native::android::ndk_sys::JavaVM = std::ptr::null_mut();
        let vm_result = unsafe { get_java_vm(env, &mut vm) };
        if vm_result != 0 || vm.is_null() {
            return;
        }

        let ctx = unsafe { new_global_ref(env, activity) };
        if ctx.is_null() {
            return;
        }

        unsafe {
            ndk_context::initialize_android_context(vm as *mut std::ffi::c_void, ctx as *mut std::ffi::c_void)
        };

        let _ = unsafe { set_android_working_dir_to_files_dir(env, activity) };
    });
}

#[cfg(target_os = "android")]
const ANDROID_REQUIRED_BUNDLED_FILES: &[(&str, &str)] = &[
    ("assets/tap.wav", "assets/tap.wav"),
    ("assets/arc.wav", "assets/arc.wav"),
    (ANDROID_BUILTIN_PROJECT_CHART, ANDROID_BUILTIN_PROJECT_CHART),
    (
        ANDROID_BUILTIN_PROJECT_IFFPROJ,
        ANDROID_BUILTIN_PROJECT_IFFPROJ,
    ),
    (ANDROID_BUILTIN_PROJECT_AUDIO, ANDROID_BUILTIN_PROJECT_AUDIO),
];

#[cfg(target_os = "android")]
const ANDROID_OPTIONAL_BUNDLED_FILES: &[(&str, &str)] = &[("assets/cjk_font.ttf", "assets/cjk_font.ttf")];

#[cfg(target_os = "android")]
async fn ensure_android_runtime_files() -> Result<(), String> {
    use std::path::Path;

    for &(asset_path, output_path) in ANDROID_REQUIRED_BUNDLED_FILES {
        let bytes = macroquad::file::load_file(asset_path)
            .await
            .map_err(|err| {
                format!(
                    "failed to read embedded asset '{asset_path}': {err}; cwd={}",
                    android_cwd_for_error()
                )
            })?;
        if let Some(parent) = Path::new(output_path).parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create '{output_path}' parent: {err}; cwd={}",
                    android_cwd_for_error()
                )
            })?;
        }
        std::fs::write(output_path, &bytes).map_err(|err| {
            format!(
                "failed to write runtime file '{output_path}': {err}; cwd={}",
                android_cwd_for_error()
            )
        })?;
    }

    for &(asset_path, output_path) in ANDROID_OPTIONAL_BUNDLED_FILES {
        let Ok(bytes) = macroquad::file::load_file(asset_path).await else {
            continue;
        };
        if let Some(parent) = Path::new(output_path).parent() {
            if std::fs::create_dir_all(parent).is_err() {
                continue;
            }
        }
        let _ = std::fs::write(output_path, &bytes);
    }

    Ok(())
}

pub async fn run_app() {
    let android_bootstrap_error: Option<String> = {
        #[cfg(target_os = "android")]
        {
            ensure_android_runtime_files().await.err()
        }
        #[cfg(not(target_os = "android"))]
        {
            None
        }
    };

    let mut i18n = I18n::from_settings(&settings().language);
    let android_boot_project = cfg!(target_os = "android")
        && std::path::Path::new(ANDROID_BUILTIN_PROJECT_CHART).is_file()
        && std::path::Path::new(ANDROID_BUILTIN_PROJECT_AUDIO).is_file();

    // On Android, always prefer built-in project bootstrap over debug DEV_MODE paths.
    let (mut editor, mut audio) = if cfg!(target_os = "android") && android_boot_project {
        (
            FallingGroundEditor::new(ANDROID_BUILTIN_PROJECT_CHART),
            AudioController::new(&i18n, ANDROID_BUILTIN_PROJECT_AUDIO),
        )
    } else if DEV_MODE {
        (
            FallingGroundEditor::new(DEV_CHART_PATH),
            AudioController::new(&i18n, DEV_AUDIO_PATH),
        )
    } else {
        (
            FallingGroundEditor::from_chart_path(""),
            AudioController::new_empty(&i18n),
        )
    };

    let mut info_toasts = InfoToastManager::new();
    let mut top_progress_state = TopProgressBarState::new();
    let mut ui = UiOrchestrator::new();
    let mut project_mgr = ProjectManager::new();
    let macroquad_font = load_macroquad_cjk_font().await;
    editor.set_text_font(macroquad_font.clone());
    apply_settings_to_editor(&mut editor, &mut audio, &i18n);

    if let Some(err) = android_bootstrap_error {
        info_toasts.push_warn(format!("android bootstrap failed: {err}"));
    }
    if macroquad_font.is_none() {
        audio.status =
            "warning: macroquad cjk font not found; Chinese text may render as tofu".to_owned();
    }
    if android_boot_project {
        info_toasts.push("android: preloaded built-in project (alamode)");
    }
    if cfg!(target_os = "android") {
        info_toasts.push(
            "gesture: single finger=left click, two-finger tap=right click, two-finger drag=wheel, four-finger tap=play/pause",
        );
    }

    loop {
        clear_background(Color::from_rgba(7, 7, 10, 255));
        refresh_ui_scale();
        update_touch_emulation();

        #[cfg(target_os = "android")]
        if android_play_pause_pressed() {
            audio.toggle_play_pause(&i18n);
        }

        // 1. Tick audio
        audio.tick(&i18n);
        let space_consumed = audio.handle_keyboard(&i18n);

        // 2. UI draw (egui)
        let ui_output = ui.draw(&mut editor, &mut audio, &i18n, &mut info_toasts);

        // 3. Menu actions
        if let Some(ref action) = ui_output.menu_action {
            audio.status.clear();
            handle_top_menu_action(
                action.clone(),
                &mut editor,
                &mut audio,
                &mut i18n,
                &mut info_toasts,
            );
            if !audio.status.is_empty() {
                info_toasts.push(audio.status.clone());
            }
        }

        // 4. Project loading
        project_mgr.handle_ui_actions(&ui_output, &mut info_toasts);
        project_mgr.tick_and_apply(
            &mut editor,
            &mut audio,
            &i18n,
            &mut info_toasts,
            &macroquad_font,
        );

        // 5. Shortcuts and wheel
        input_handler::handle_shortcuts(&mut editor, &mut audio, &i18n, &mut info_toasts);
        input_handler::handle_wheel(
            &mut editor,
            &mut audio,
            &i18n,
            &mut info_toasts,
            space_consumed,
            &ui_output,
        );

        // 6. Layout
        let mut frame_ctx = audio.frame_snapshot();
        let ui_scale = ui_output.ui_scale;
        let menu_height = ui_output.menu_height;
        let top_bar_height = ui_output.top_bar_height;
        let panel_pad = 10.0 * ui_scale;
        let editor_gap = 12.0 * ui_scale;
        let editor_bottom_pad = 8.0 * ui_scale;
        let editor_width =
            (screen_width() - panel_pad * 2.0 - ui_output.total_right_panels_px - editor_gap)
                .max(360.0);

        // 7. Top progress bar
        let progress_output = draw_top_progress_bar(
            ui_scale,
            menu_height,
            top_bar_height,
            ui_output.note_panel_width_px,
            &frame_ctx,
            macroquad_font.as_ref(),
            &mut top_progress_state,
        );
        frame_ctx.current_sec = progress_output.display_sec;
        if let Some(seek_sec) = progress_output.seek_to_sec {
            audio.handle_editor_seek(seek_sec, &i18n);
            frame_ctx.current_sec = audio.current_sec();
        }

        // 8. Editor draw
        let editor_y = menu_height + top_bar_height + 8.0 * ui_scale;
        let editor_rect = Rect::new(
            panel_pad,
            editor_y,
            editor_width,
            (screen_height() - editor_y - editor_bottom_pad).max(140.0),
        );
        for action in editor.draw(editor_rect, &frame_ctx) {
            match action {
                FallingEditorAction::SeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                    info_toasts.push(format!("seek {:.2}s", sec));
                }
                FallingEditorAction::MinimapSeekTo(sec) => {
                    audio.handle_editor_seek(sec, &i18n);
                }
            }
        }
        for (msg, is_warn) in editor.drain_toasts() {
            if is_warn {
                info_toasts.push_warn(&msg);
            } else {
                info_toasts.push(&msg);
            }
        }

        // 9. Hitsound
        {
            let note_heads = editor.note_head_times();
            audio.trigger_hitsounds(&note_heads);
        }

        // 10. Toasts
        info_toasts.draw(
            ui_scale,
            menu_height + top_bar_height,
            macroquad_font.as_ref(),
        );

        egui_macroquad::draw();
        next_frame().await;
    }
}
