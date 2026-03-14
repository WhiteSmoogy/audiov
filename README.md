# audiov

> 让语音输入像键盘操作一样直接。

`audiov`（Audio + V）是一个使用 Rust 编写的 Linux 全局语音输入守护进程，通过 `fcitx5` addon 将识别结果直接提交到当前输入上下文。

## 核心特性

- **本地离线识别**：基于 `whisper.cpp`，支持在本机 CPU/GPU 上完成语音转写，数据不离开设备。
- **远端 Whisper 接口**：支持通过配置切换到 Whisper 兼容的远端 API。
- **fcitx5 直接注入**：通过 `fcitx5` addon 的 `CommitText` DBus 方法，将文本直接提交给当前聚焦输入上下文。
- **KDE 全局快捷键**：通过 `org.kde.KGlobalAccel` 注册全局快捷键，默认 `Meta+H` 单键切换录音。
- **手动录音模式**：保留前台交互录音模式，便于调试与验证链路。

## 工作流程

1. **Capture**：监听 KDE 全局快捷键或手动触发，采集麦克风音频（PipeWire/PulseAudio/ALSA）。
2. **Inference**：调用本地 `whisper.cpp` 引擎或远端 HTTP API 完成语音转文本。
3. **Inject**：通过 `fcitx5` addon DBus 接口将文本直接提交到当前输入上下文。

## 快速开始

### 依赖项

- Rust 工具链（`cargo`）
- `fcitx5`（含 audiov addon，见 `contrib/fcitx5-audiov/`）
- KDE Plasma（全局快捷键模式）

### 安装 fcitx5 addon

```bash
./contrib/fcitx5-audiov/install-user.sh
./contrib/fcitx5-audiov/restart-fcitx5.sh
```

验证 addon 已加载：

```bash
busctl --user introspect org.fcitx.Fcitx5 /org/freedesktop/Fcitx5/Audiov
```

### 编译与运行

```bash
cargo build --release

# KDE 全局快捷键模式（默认）
./target/release/audiov --config ~/.config/audiov/config.toml

# 手动录音模式
./target/release/audiov --manual --config ~/.config/audiov/config.toml
```

配置文件默认查找顺序：

1. `AUDIOV_CONFIG` 环境变量；
2. `~/.config/audiov/config.toml`（若存在）；
3. 若均不存在则报错，需通过 `--config` 显式指定。

### 作为系统服务运行

```bash
# 创建 ~/.config/systemd/user/audiov.service
systemctl --user enable --now audiov.service

# 常用命令
systemctl --user status audiov
journalctl --user -fu audiov
```

## 配置

参见 `config.example.toml`，主要配置段：

### 转写后端

```toml
[whisper]
backend = "cpp"  # 或 "remote"

[whisper_cpp]
model = "models/ggml-large-v1.bin"
threads = 4
use_gpu = false

[whisper_remote]
enabled = false
endpoint = "https://api.openai.com/v1/audio/transcriptions"
model = "whisper-1"
api_key = ""
timeout_secs = 60
```

### fcitx5 输出

```toml
[paste]
fcitx5_service = "org.fcitx.Fcitx5"
fcitx5_path = "/org/freedesktop/Fcitx5/Audiov"
fcitx5_interface = "org.fcitx.Fcitx5.Audiov1"
```

### KDE 全局快捷键

```toml
[hotkey]
enabled = true
shortcut = "Meta+H"
component_unique = "audiov"
component_friendly = "audiov"
action_unique = "toggle-recording"
action_friendly = "Toggle Recording"
```

第一次按下快捷键开始录音，再次按下停止并转写注入。

### 语言识别（LID）

```toml
[language_detection]
enabled = false
mode = "per_session"
allowed_languages = ["zh", "en"]
confidence_threshold = 0.65
default_language = "zh"
use_detected_language_for_inference = false
```

### 录音后端

```toml
[recorder]
backend = "auto"  # auto | pipewire | pulseaudio | alsa
# input_device = ""
```

## 启动参数

| 参数 | 说明 |
|------|------|
| `--config <path>` | 指定配置文件路径 |
| `--manual` | 前台手动模式，按 Enter 开始/停止录音 |
| `--transcribe-wav <path>` | 直接转写 16kHz/mono/PCM16 WAV 文件 |
