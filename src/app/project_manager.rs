use crate::app::setup::apply_settings_to_editor;
use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::i18n::I18n;
use crate::ui::current_project_window::CurrentProjectAction;
use crate::ui::info_toast::InfoToastManager;
use crate::ui::loading_status::{LoadAction, ProjectLoader};
use macroquad::prelude::Font;
use std::path::{Path, PathBuf};

use super::ui_orchestrator::UiOutput;

pub struct ProjectManager {
    loader: ProjectLoader,
    project_path: Option<PathBuf>,
    pending_project_path: Option<PathBuf>,
    pending_asset: Option<CurrentProjectAction>,
    pending_music_time_ms: Option<f32>,
}

impl ProjectManager {
    pub fn new() -> Self {
        Self {
            loader: ProjectLoader::new(),
            project_path: None,
            pending_project_path: None,
            pending_asset: None,
            pending_music_time_ms: None,
        }
    }

    pub fn save_music_time_to_project(&self, _editor: &FallingGroundEditor, audio: &AudioController) -> Result<(), String> {
        // A loose chart has no project file to update.
        let Some(path) = &self.project_path else { return Ok(()); };
        let mut json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).map_err(|e| format!("读取工程文件失败: {e}"))?
        ).map_err(|e| format!("解析工程文件失败: {e}"))?;
        json["last_music_time_ms"] = serde_json::json!(audio.current_sec() * 1000.0);
        std::fs::write(path, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
            .map_err(|e| format!("写入工程文件失败: {e}"))
    }

    pub fn save_project(&mut self, editor: &mut FallingGroundEditor, audio: &AudioController) -> Result<Option<String>, String> {
        if self.loader.is_loading() || self.pending_asset.is_some() {
            return Err("请等待加载完成后再保存".to_owned());
        }
        if editor.chart_path().is_empty() && audio.track_path().is_none() && !editor.is_dirty() {
            return Err("请先加载音频、谱面或编辑谱面".to_owned());
        }
        let path = match &self.project_path {
            Some(path) => path.clone(),
            None => {
                let name = Path::new(editor.chart_path()).file_stem()
                    .or_else(|| audio.track_path().and_then(|p| Path::new(p).file_stem()))
                    .unwrap_or(std::ffi::OsStr::new("project"));
                let Some(path) = rfd::FileDialog::new()
                    .add_filter("IFF Project", &["iffproj"])
                    .set_file_name(format!("{}.iffproj", name.to_string_lossy()))
                    .save_file() else { return Ok(None); };
                path
            }
        };
        let audio_path = audio.track_path().unwrap_or("");
        if path.extension().and_then(|e| e.to_str()) != Some("iffproj") {
            return Err("项目文件请使用 .iffproj 扩展名".to_owned());
        }
        if !audio_path.is_empty() && !Path::new(audio_path).is_file() {
            return Err("音频文件不存在，请重新加载音频".to_owned());
        }
        editor.save_chart()?;
        write_project_file(&path, editor.chart_path(), audio_path, audio.current_sec() * 1000.0)?;
        self.project_path = Some(path.clone());
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    pub fn handle_ui_actions(&mut self, ui: &UiOutput, info_toasts: &mut InfoToastManager) {
        if ui.open_project.is_none() && ui.current_project_action.is_none() && ui.create_project.is_none() {
            return;
        }
        if self.loader.is_loading() || self.pending_asset.is_some() {
            info_toasts.push_warn("请等待当前加载完成");
            return;
        }
        if let Some((chart_path, audio_path, time, project_path)) = &ui.open_project {
            self.pending_project_path = Some(PathBuf::from(project_path));
            self.loader.start_open_project(chart_path.clone(), audio_path.clone(), *time);
        } else if let Some(action) = &ui.current_project_action {
            self.pending_asset = Some(action.clone());
        } else if let Some(params) = &ui.create_project {
            self.pending_project_path = Some(PathBuf::from(format!("projects/{0}/{0}.iffproj", params.name)));
            self.loader.start_create_project(params.name.clone(), params.source_audio.clone(), params.bpm, params.bpl);
        }
        if self.loader.is_loading() {
            info_toasts.pin(self.loader.status_text());
        }
    }

    pub fn tick_and_apply(
        &mut self,
        editor: &mut FallingGroundEditor,
        audio: &mut AudioController,
        i18n: &I18n,
        info_toasts: &mut InfoToastManager,
        macroquad_font: &Option<Font>,
    ) {
        if let Some(action) = self.pending_asset.take() {
            match action {
                CurrentProjectAction::LoadChart(path) => {
                    match replace_chart(&path, editor, audio, i18n, macroquad_font) {
                        Ok(true) => {
                            self.project_path = None;
                            info_toasts.push(format!("谱面已加载: {path}"));
                        }
                        Ok(false) => {},
                        Err(e) => info_toasts.push_warn(format!("加载谱面失败: {e}")),
                    }
                }
                CurrentProjectAction::LoadAudio(path) => {
                    // ponytail: reuse the decoder without ever rebuilding the editor.
                    self.loader.advance_after_chart_load(String::new(), path);
                    info_toasts.pin(self.loader.status_text());
                }
            }
        }
        let prev_status = self.loader.status_text().to_owned();
        let action = self.loader.tick();
        let new_status = self.loader.status_text();
        if new_status != prev_status {
            if new_status.is_empty() {
                info_toasts.dismiss_pinned();
            } else {
                info_toasts.pin(new_status);
            }
        }
        match action {
            LoadAction::None => {}
            LoadAction::LoadChart { chart_path, audio_path, last_music_time_ms } => {
                match replace_chart(&chart_path, editor, audio, i18n, macroquad_font) {
                    Ok(true) => {},
                    result => {
                        self.loader.finish();
                        self.pending_project_path = None;
                        info_toasts.dismiss_pinned();
                        if let Err(e) = result { info_toasts.push_warn(format!("加载谱面失败: {e}")); }
                        return;
                    }
                }
                self.project_path = self.pending_project_path.take();
                // Opening a project replaces the whole session, including missing audio.
                audio.pause_if_playing(i18n);
                *audio = AudioController::new_empty(i18n);
                apply_settings_to_editor(editor, audio, i18n);
                if audio_path.is_empty() {
                    self.loader.finish();
                    info_toasts.dismiss_pinned();
                    info_toasts.push(format!("项目已加载: {chart_path}"));
                } else {
                    self.pending_music_time_ms = Some(last_music_time_ms);
                    self.loader.advance_after_chart_load(chart_path, audio_path);
                    info_toasts.pin(self.loader.status_text());
                }
            }
            LoadAction::InstallAudio { clip, chart_path, audio_path } => {
                self.loader.finish();
                info_toasts.dismiss_pinned();
                let saved_time = self.pending_music_time_ms.take();
                if let Err(e) = audio.install_decoded_audio(clip, &audio_path, i18n) {
                    info_toasts.push_warn(format!("加载音频失败: {e}"));
                    return;
                }
                if let Some(time) = saved_time {
                    audio.handle_editor_seek((time / 1000.0).max(0.0), i18n);
                }
                if chart_path.is_empty() {
                    self.project_path = None;
                    info_toasts.push(format!("音频已加载: {audio_path}"));
                } else {
                    info_toasts.push(format!("项目已加载: {chart_path}"));
                }
            }
            LoadAction::Error(e) => {
                self.loader.finish();
                self.pending_project_path = None;
                self.pending_music_time_ms = None;
                info_toasts.dismiss_pinned();
                info_toasts.push_warn(format!("加载失败: {e}"));
            }
        }
    }
}

fn replace_chart(
    path: &str,
    editor: &mut FallingGroundEditor,
    audio: &mut AudioController,
    i18n: &I18n,
    font: &Option<Font>,
) -> Result<bool, String> {
    // Validate before touching the current chart or asking to discard edits.
    let mut loaded = FallingGroundEditor::try_from_chart_path(path)?;
    if editor.is_dirty() {
        let was_playing = audio.pause_if_playing(i18n);
        let discard = rfd::MessageDialog::new()
            .set_title("未保存的谱面")
            .set_description("当前谱面有未保存的修改。放弃修改并加载新谱面？")
            .set_buttons(rfd::MessageButtons::YesNo)
            .set_level(rfd::MessageLevel::Warning)
            .show() == rfd::MessageDialogResult::Yes;
        audio.resume_if_was_playing(was_playing, i18n);
        if !discard { return Ok(false); }
    }
    loaded.set_text_font(font.clone());
    apply_settings_to_editor(&mut loaded, audio, i18n);
    *editor = loaded;
    Ok(true)
}

pub(crate) fn read_project_file(path: &Path) -> Result<(String, String, f32), String> {
    let json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(path).map_err(|e| format!("读取 iffproj 失败: {e}"))?
    ).map_err(|e| format!("解析 iffproj 失败: {e}"))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let resolve = |key: &str| -> Result<String, String> {
        let value = json.get(key).and_then(|v| v.as_str())
            .ok_or_else(|| format!("iffproj 缺少 {key} 字段"))?;
        if value.is_empty() { return Ok(String::new()); }
        Ok(dir.join(value).to_string_lossy().into_owned())
    };
    let chart = resolve("chart_path")?;
    if chart.is_empty() { return Err("iffproj 的谱面路径为空".to_owned()); }
    let time = json.get("last_music_time_ms").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    Ok((chart, resolve("audio_path")?, if time.is_finite() { time.max(0.0) } else { 0.0 }))
}

fn write_project_file(path: &Path, chart: &str, audio: &str, time_ms: f32) -> Result<(), String> {
    let absolute_path = std::path::absolute(path).map_err(|e| e.to_string())?;
    let dir = absolute_path.parent().ok_or("项目路径无效")?
        .canonicalize().map_err(|e| e.to_string())?;
    let asset_path = |value: &str| -> Result<String, String> {
        if value.is_empty() { return Ok(String::new()); }
        let absolute = Path::new(value).canonicalize().map_err(|e| format!("资源文件不存在: {e}"))?;
        // ponytail: relative inside the project folder, absolute elsewhere; no asset copying.
        Ok(absolute.strip_prefix(&dir).unwrap_or(&absolute).to_string_lossy().into_owned())
    };
    let json = serde_json::json!({
        "chart_path": asset_path(chart)?,
        "audio_path": asset_path(audio)?,
        "last_music_time_ms": time_ms,
    });
    std::fs::write(path, serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?)
        .map_err(|e| format!("写入工程文件失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loose_assets_project_round_trip() {
        let dir = std::env::temp_dir().join(format!("iff-project-test-{}-{}", std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir_all(dir.join("assets")).unwrap();
        let chart = dir.join("assets/chart.spc");
        let audio = dir.join("music.wav");
        let project = dir.join("different-name.iffproj");
        std::fs::write(&chart, "chart(120,4)\n").unwrap();
        std::fs::write(&audio, b"audio placeholder").unwrap();
        let mut editor = FallingGroundEditor::try_from_chart_path(chart.to_str().unwrap()).unwrap();
        let snapshot = editor.to_chart().to_spc();
        std::fs::write(&chart, "chart(180,4)\n").unwrap();
        editor.save_chart().unwrap();
        assert_eq!(std::fs::read_to_string(&chart).unwrap(), snapshot);
        write_project_file(&project, chart.to_str().unwrap(), audio.to_str().unwrap(), 1250.0).unwrap();
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&project).unwrap()).unwrap();
        assert!(!Path::new(json["chart_path"].as_str().unwrap()).is_absolute());
        let (cp, ap, time) = read_project_file(&project).unwrap();
        assert_eq!(Path::new(&cp).canonicalize().unwrap(), chart.canonicalize().unwrap());
        assert_eq!(Path::new(&ap).canonicalize().unwrap(), audio.canonicalize().unwrap());
        assert_eq!(time, 1250.0);
        write_project_file(&project, chart.to_str().unwrap(), "", 0.0).unwrap();
        assert_eq!(read_project_file(&project).unwrap().1, "");
        let outside_project = dir.join("assets/another.iffproj");
        write_project_file(&outside_project, chart.to_str().unwrap(), audio.to_str().unwrap(), 0.0).unwrap();
        let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&outside_project).unwrap()).unwrap();
        assert!(Path::new(json["audio_path"].as_str().unwrap()).is_absolute());
        assert!(write_project_file(&project, "missing-chart.spc", "", 0.0).is_err());
        assert!(FallingGroundEditor::try_from_chart_path(audio.to_str().unwrap()).is_err());
        std::fs::write(&project, "{}").unwrap();
        assert!(read_project_file(&project).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
