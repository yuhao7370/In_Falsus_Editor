use sasa::AudioClip;
use std::path::Path;
use std::sync::mpsc;
use std::thread;

/// 异步项目加载状态机。
/// 将耗时操作（复制文件、读取音频字节、解码音频）放到后台线程，
/// 每帧轮询进度并在左下角显示当前步骤。
pub struct ProjectLoader {
    phase: LoadPhase,
    status_text: String,
}

/// 后台线程结果接收器
struct BgTask<T> {
    rx: mpsc::Receiver<T>,
}

impl<T> BgTask<T> {
    fn try_recv(&self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

enum LoadPhase {
    /// 空闲，无加载任务
    Idle,

    // ── 创建项目流程 ──

    /// 阶段1: 后台复制音频文件 + 创建项目文件
    CreateCopyAudio {
        task: BgTask<Result<(String, String), String>>,
    },
    /// 阶段2: 主线程加载谱面到编辑器（返回 LoadChart 动作后等待 advance）
    CreateLoadChart {
        chart_path: String,
        audio_path: String,
    },
    /// 阶段3: 后台读取音频字节 + 解码为 AudioClip
    CreateDecodeAudioBg {
        task: BgTask<Result<(AudioClip, String), String>>,
        chart_path: String,
    },

    // ── 打开项目流程 ──

    /// 阶段1: 主线程加载谱面（返回 LoadChart 动作后等待 advance）
    OpenLoadChart {
        chart_path: String,
        audio_path: String,
    },
}

/// tick() 返回的动作，告诉 main 该做什么
pub enum LoadAction {
    /// 无事发生，继续等待
    None,
    /// 请求主线程加载谱面
    LoadChart { chart_path: String, audio_path: String },
    /// 后台解码完成，请求主线程安装已解码的 AudioClip（不阻塞）
    InstallAudio { clip: AudioClip, chart_path: String, audio_path: String },
    /// 加载出错
    Error(String),
}

impl ProjectLoader {
    pub fn new() -> Self {
        Self {
            phase: LoadPhase::Idle,
            status_text: String::new(),
        }
    }

    pub fn is_loading(&self) -> bool {
        !matches!(self.phase, LoadPhase::Idle)
    }

    /// 启动"创建项目"异步流程
    pub fn start_create_project(
        &mut self,
        name: String,
        source_audio: String,
        bpm: f64,
        bpl: f64,
    ) {
        self.status_text = "创建项目文件 & 复制音频...".to_string();
        let (tx, rx) = mpsc::channel();
        let name_clone = name.clone();
        let source_clone = source_audio.clone();
        thread::spawn(move || {
            let result = create_project_on_disk_sync(&name_clone, &source_clone, bpm, bpl);
            let _ = tx.send(result);
        });
        self.phase = LoadPhase::CreateCopyAudio {
            task: BgTask { rx },
        };
    }

    /// 启动"打开项目"异步流程
    pub fn start_open_project(&mut self, chart_path: String, audio_path: String) {
        self.status_text = "加载谱面...".to_string();
        self.phase = LoadPhase::OpenLoadChart { chart_path, audio_path };
    }

    /// 每帧调用，推进状态机。返回需要主线程执行的动作。
    pub fn tick(&mut self) -> LoadAction {
        let phase = std::mem::replace(&mut self.phase, LoadPhase::Idle);
        match phase {
            LoadPhase::Idle => {
                self.phase = LoadPhase::Idle;
                LoadAction::None
            }

            // ── 创建项目 ──

            LoadPhase::CreateCopyAudio { task } => {
                if let Some(result) = task.try_recv() {
                    match result {
                        Ok((chart_path, audio_path)) => {
                            self.status_text = "加载谱面...".to_string();
                            self.phase = LoadPhase::CreateLoadChart {
                                chart_path: chart_path.clone(),
                                audio_path: audio_path.clone(),
                            };
                            LoadAction::LoadChart { chart_path, audio_path }
                        }
                        Err(e) => {
                            self.status_text.clear();
                            self.phase = LoadPhase::Idle;
                            LoadAction::Error(e)
                        }
                    }
                } else {
                    self.phase = LoadPhase::CreateCopyAudio { task };
                    LoadAction::None
                }
            }

            LoadPhase::CreateLoadChart { chart_path, audio_path } => {
                // 等待主线程调用 advance_after_chart_load()
                self.phase = LoadPhase::CreateLoadChart { chart_path, audio_path };
                LoadAction::None
            }

            LoadPhase::CreateDecodeAudioBg { task, chart_path } => {
                if let Some(result) = task.try_recv() {
                    match result {
                        Ok((clip, audio_path)) => {
                            self.status_text.clear();
                            self.phase = LoadPhase::Idle;
                            LoadAction::InstallAudio { clip, chart_path, audio_path }
                        }
                        Err(e) => {
                            self.status_text.clear();
                            self.phase = LoadPhase::Idle;
                            LoadAction::Error(e)
                        }
                    }
                } else {
                    self.phase = LoadPhase::CreateDecodeAudioBg { task, chart_path };
                    LoadAction::None
                }
            }

            // ── 打开项目 ──

            LoadPhase::OpenLoadChart { chart_path, audio_path } => {
                self.status_text = "加载谱面...".to_string();
                self.phase = LoadPhase::Idle;
                LoadAction::LoadChart { chart_path, audio_path }
            }

        }
    }

    /// 主线程加载完谱面后调用，启动后台线程读取+解码音频
    pub fn advance_after_chart_load(&mut self, chart_path: String, audio_path: String) {
        self.status_text = "读取并解码音频...".to_string();
        let path_clone = audio_path.clone();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let result = read_and_decode_audio(&path_clone);
            let _ = tx.send(result);
        });
        // 判断当前是创建还是打开流程，都用对应的 Bg 阶段
        self.phase = LoadPhase::CreateDecodeAudioBg {
            task: BgTask { rx },
            chart_path,
        };
    }

    /// 加载完成，清除状态
    pub fn finish(&mut self) {
        self.status_text.clear();
        self.phase = LoadPhase::Idle;
    }

    /// 当前加载状态文本（空字符串表示无加载任务）
    pub fn status_text(&self) -> &str {
        &self.status_text
    }
}

/// 后台线程：读取音频文件字节 + 解码为 AudioClip
fn read_and_decode_audio(path: &str) -> Result<(AudioClip, String), String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("读取音频文件失败: {e}"))?;
    let clip = AudioClip::new(bytes)
        .map_err(|e| format!("解码音频失败: {e}"))?;
    Ok((clip, path.to_string()))
}

/// 同步创建项目文件（在后台线程中执行）
fn create_project_on_disk_sync(
    name: &str,
    source_audio: &str,
    bpm: f64,
    bpl: f64,
) -> Result<(String, String), String> {
    let project_dir = format!("projects/{}", name);
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("创建项目目录失败: {e}"))?;

    // Copy audio file
    let audio_source = Path::new(source_audio);
    let audio_ext = audio_source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("ogg");
    let audio_filename = format!("music.{}", audio_ext);
    let audio_dest = format!("{}/{}", project_dir, audio_filename);
    std::fs::copy(source_audio, &audio_dest)
        .map_err(|e| format!("复制音频文件失败: {e}"))?;

    // Create .spc file
    let chart_filename = format!("{}.spc", name);
    let chart_path = format!("{}/{}", project_dir, chart_filename);
    let spc_content = format!("chart({:.2},{:.2})\n", bpm, bpl);
    std::fs::write(&chart_path, &spc_content)
        .map_err(|e| format!("创建谱面文件失败: {e}"))?;

    // Create .iffproj file (paths relative to the .iffproj file's directory)
    let proj_path = format!("{}/{}.iffproj", project_dir, name);
    let proj_json = serde_json::json!({
        "audio_path": audio_filename,
        "chart_path": chart_filename,
    });
    let proj_content = serde_json::to_string_pretty(&proj_json)
        .map_err(|e| format!("序列化项目文件失败: {e}"))?;
    std::fs::write(&proj_path, &proj_content)
        .map_err(|e| format!("创建项目文件失败: {e}"))?;

    Ok((chart_path, audio_dest))
}
