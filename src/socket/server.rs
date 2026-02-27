use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use tungstenite::protocol::Message;
use tungstenite::{accept, WebSocket};

use crate::audio::controller::AudioController;
use crate::editor::falling::FallingGroundEditor;
use crate::settings;

// ═══════════════════════════════════════════════════════════════
// 可调参数：播放时时间+流速的最大发送频率（每秒次数）
// ═══════════════════════════════════════════════════════════════
const CHART_SEND_INTERVAL_SECS: f64 = 1.0;
const LISTEN_PORT: u16 = 11451;

/// 从主线程推送给广播线程的消息。
enum BroadcastMsg {
    /// 完整信息（响应 getinfo）— JSON 字符串
    FullInfo(String),
    /// 谱面变动 — JSON 字符串
    ChartUpdate(String),
    /// 播放状态（时间+流速）— JSON 字符串
    Playback(String),
    /// 新客户端连接
    NewClient(WebSocket<TcpStream>),
}

/// 从广播线程发回主线程的请求。
enum ClientRequest {
    GetInfo,
}

pub struct SocketServer {
    broadcast_tx: Option<mpsc::Sender<BroadcastMsg>>,
    request_rx: Option<mpsc::Receiver<ClientRequest>>,

    // ── 状态跟踪 ──
    last_chart_json: Option<String>,
    pending_chart_json: Option<String>,
    last_chart_send: Instant,
    last_sent_time: f32,
    last_sent_speed: f32,
    last_playback_send: Instant,
    playback_interval_secs: f64,
    enabled: bool,
}

/// 获取本机局域网 IP 地址（通过 UDP 连接技巧，不实际发送数据）
fn get_local_ip() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_owned())
}

impl SocketServer {
    pub fn new() -> Self {
        let enabled = settings::settings().socket_server_enabled;

        if !enabled {
            println!("[socket] Socket server is disabled in settings");
            return Self {
                broadcast_tx: None,
                request_rx: None,
                last_chart_json: None,
                pending_chart_json: None,
                last_chart_send: Instant::now(),
                last_sent_time: f32::NAN,
                last_sent_speed: f32::NAN,
                last_playback_send: Instant::now(),
                playback_interval_secs: 1.0,
                enabled: false,
            };
        }

        let (broadcast_tx, broadcast_rx) = mpsc::channel::<BroadcastMsg>();
        let (request_tx, request_rx) = mpsc::channel::<ClientRequest>();

        // ── 接受连接线程 ──
        let accept_tx = broadcast_tx.clone();
        let accept_req_tx = request_tx.clone();
        thread::spawn(move || {
            Self::accept_loop(accept_tx, accept_req_tx);
        });

        // ── 广播线程 ──
        thread::spawn(move || {
            Self::broadcast_loop(broadcast_rx, request_tx);
        });

        Self {
            broadcast_tx: Some(broadcast_tx),
            request_rx: Some(request_rx),
            last_chart_json: None,
            pending_chart_json: None,
            last_chart_send: Instant::now(),
            last_sent_time: f32::NAN,
            last_sent_speed: f32::NAN,
            last_playback_send: Instant::now(),
            playback_interval_secs: 1.0,
            enabled: true,
        }
    }

    /// 主循环每帧调用。
    pub fn tick(
        &mut self,
        editor: &FallingGroundEditor,
        audio: &AudioController,
    ) {
        if !self.enabled {
            return;
        }
        let broadcast_tx = match &self.broadcast_tx {
            Some(tx) => tx,
            None => return,
        };
        let request_rx = match &self.request_rx {
            Some(rx) => rx,
            None => return,
        };
        let playback_send_rate = settings::AppSettings::clamp_socket_playback_send_rate(
            settings::settings().socket_playback_send_rate,
        );
        self.playback_interval_secs = 1.0 / playback_send_rate as f64;

        // 1. 处理客户端 getinfo 请求
        while let Ok(req) = request_rx.try_recv() {
            match req {
                ClientRequest::GetInfo => {
                    let json = self.build_full_info(editor, audio);
                    let _ = broadcast_tx.send(BroadcastMsg::FullInfo(json));
                }
            }
        }

        // 2. 谱面变动检测：dirty 时更新 pending，按 1Hz 发送并保证发送最新值
        if editor.is_dirty() {
            let chart = editor.to_chart();
            let chart_spc = chart.to_spc();
            let changed = match &self.last_chart_json {
                Some(prev) => prev != &chart_spc,
                None => true,
            };
            if changed {
                self.pending_chart_json = Some(chart_spc);
            }
        }

        if let Some(chart_spc) = self.pending_chart_json.as_ref() {
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_chart_send).as_secs_f64();
            if elapsed >= CHART_SEND_INTERVAL_SECS {
                let chart_escaped = serde_json::to_string(chart_spc).unwrap_or_default();
                let msg = format!(r#"{{"type":"chart","chart":{}}}"#, chart_escaped);
                let _ = broadcast_tx.send(BroadcastMsg::ChartUpdate(msg));
                self.last_chart_json = Some(chart_spc.clone());
                self.pending_chart_json = None;
                self.last_chart_send = now;
            }
        }

        // 3. 播放状态
        let current_sec = audio.current_sec();
        let speed = editor.scroll_speed();
        let is_playing = audio.is_playing();

        if is_playing {
            // 播放中：按频率限制发送
            let now = Instant::now();
            let elapsed = now.duration_since(self.last_playback_send).as_secs_f64();
            if elapsed >= self.playback_interval_secs {
                let msg = format!(
                    r#"{{"type":"playback","time":{},"speed":{}}}"#,
                    current_sec, speed
                );
                let _ = broadcast_tx.send(BroadcastMsg::Playback(msg));
                self.last_sent_time = current_sec;
                self.last_sent_speed = speed;
                self.last_playback_send = now;
            }
        } else {
            // 暂停中：只在时间变化时发送
            let time_changed = (current_sec - self.last_sent_time).abs() > 0.0001
                || self.last_sent_time.is_nan();
            if time_changed {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_playback_send).as_secs_f64();
                if elapsed >= self.playback_interval_secs {
                    let msg = format!(
                        r#"{{"type":"playback","time":{},"speed":{}}}"#,
                        current_sec, speed
                    );
                    let _ = broadcast_tx.send(BroadcastMsg::Playback(msg));
                    self.last_sent_time = current_sec;
                    self.last_sent_speed = speed;
                    self.last_playback_send = now;
                }
            }
        }
    }

    // ── 构建完整信息 JSON ──

    fn build_full_info(
        &self,
        editor: &FallingGroundEditor,
        audio: &AudioController,
    ) -> String {
        let s = settings::settings();
        let settings_json = serde_json::to_value(&*s).unwrap_or_default();
        drop(s);

        let chart = editor.to_chart();
        let chart_spc = chart.to_spc();

        let info = serde_json::json!({
            "type": "info",
            "settings": settings_json,
            "chart": chart_spc,
            "chart_path": editor.chart_path(),
            "is_playing": audio.is_playing(),
            "current_time": audio.current_sec(),
            "duration": audio.duration_sec(),
            "scroll_speed": editor.scroll_speed(),
            "snap_division": editor.snap_division(),
            "track_speed_enabled": editor.track_speed_enabled(),
        });

        info.to_string()
    }

    // ── 接受连接循环（独立线程）──

    fn accept_loop(
        broadcast_tx: mpsc::Sender<BroadcastMsg>,
        _request_tx: mpsc::Sender<ClientRequest>,
    ) {
        let bind_addr = format!("0.0.0.0:{}", LISTEN_PORT);
        let listener = match TcpListener::bind(&bind_addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[socket] failed to bind {}: {e}", bind_addr);
                return;
            }
        };
        let local_ip = get_local_ip();
        println!("[socket] WebSocket server listening on ws://{}:{}", local_ip, LISTEN_PORT);

        for stream in listener.incoming() {
            match stream {
                Ok(tcp) => {
                    let tx = broadcast_tx.clone();
                    thread::spawn(move || {
                        match accept(tcp) {
                            Ok(ws) => {
                                println!("[socket] client connected");
                                let _ = tx.send(BroadcastMsg::NewClient(ws));
                            }
                            Err(e) => {
                                eprintln!("[socket] handshake failed: {e}");
                            }
                        }
                    });
                }
                Err(e) => {
                    eprintln!("[socket] accept error: {e}");
                }
            }
        }
    }

    // ── 广播循环（独立线程）──
    // 管理所有客户端 WebSocket，处理读写。
    // 使用 recv_timeout 避免阻塞，确保能定期轮询客户端消息。

    fn broadcast_loop(
        rx: mpsc::Receiver<BroadcastMsg>,
        request_tx: mpsc::Sender<ClientRequest>,
    ) {
        let mut clients: Vec<WebSocket<TcpStream>> = Vec::new();
        let poll_interval = Duration::from_millis(10);

        loop {
            // 使用超时接收，确保即使没有广播消息也能定期轮询客户端
            match rx.recv_timeout(poll_interval) {
                Ok(msg) => {
                    Self::handle_broadcast_msg(msg, &mut clients);

                    // 处理同一批次的剩余消息
                    while let Ok(msg) = rx.try_recv() {
                        Self::handle_broadcast_msg(msg, &mut clients);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // 超时 — 正常，继续轮询客户端消息
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // 发送端全部断开，退出
                    break;
                }
            }

            // 每次循环都轮询客户端入站消息
            if !clients.is_empty() {
                Self::poll_client_messages(&mut clients, &request_tx);
            }
        }
    }

    fn handle_broadcast_msg(
        msg: BroadcastMsg,
        clients: &mut Vec<WebSocket<TcpStream>>,
    ) {
        match msg {
            BroadcastMsg::NewClient(ws) => {
                if let Ok(raw) = ws.get_ref().try_clone() {
                    let _ = raw.set_nonblocking(true);
                }
                clients.push(ws);
            }
            BroadcastMsg::FullInfo(json)
            | BroadcastMsg::ChartUpdate(json)
            | BroadcastMsg::Playback(json) => {
                Self::broadcast_to_clients(clients, &json);
            }
        }
    }

    fn broadcast_to_clients(clients: &mut Vec<WebSocket<TcpStream>>, json: &str) {
        let mut dead = Vec::new();
        for (i, ws) in clients.iter_mut().enumerate() {
            if ws.write(Message::Text(json.to_owned())).is_err() || ws.flush().is_err() {
                dead.push(i);
            }
        }
        for i in dead.into_iter().rev() {
            println!("[socket] client disconnected");
            clients.swap_remove(i);
        }
    }

    fn poll_client_messages(
        clients: &mut Vec<WebSocket<TcpStream>>,
        request_tx: &mpsc::Sender<ClientRequest>,
    ) {
        let mut dead = Vec::new();
        for (i, ws) in clients.iter_mut().enumerate() {
            loop {
                match ws.read() {
                    Ok(Message::Text(text)) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if val.get("type").and_then(|v| v.as_str()) == Some("getinfo") {
                                let _ = request_tx.send(ClientRequest::GetInfo);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        dead.push(i);
                        break;
                    }
                    Ok(_) => {}
                    Err(tungstenite::Error::Io(ref e))
                        if e.kind() == std::io::ErrorKind::WouldBlock =>
                    {
                        break;
                    }
                    Err(_) => {
                        dead.push(i);
                        break;
                    }
                }
            }
        }
        for i in dead.into_iter().rev() {
            println!("[socket] client disconnected (read)");
            clients.swap_remove(i);
        }
    }
}
