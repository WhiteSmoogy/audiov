# audiov

> 让语音输入像键盘操作一样直接。

`audiov`（Audio + V）是一个使用 Rust 编写的 Linux 全局语音输入守护进程。它坚持本地优先、无云依赖，并通过“剪贴板 + 虚拟输入设备”的方式，在不同桌面协议下稳定完成文本输入。

## 核心特性

- **本地离线识别**：基于 `whisper.cpp`，支持在本机 CPU/GPU 上完成语音转写，数据不离开设备。
- **远端 Whisper 接口**：支持通过配置切换到 Whisper 兼容的远端 API，并在配置文件中提供 `api_key`。
- **稳定文本注入**：支持通过系统剪贴板 + 粘贴命令，或通过 `fcitx5` addon 直接向当前输入上下文提交文本。
- **兼容 Wayland / X11**：避免依赖脆弱的图形层模拟，适配主流 Linux 桌面环境。
- **KDE 全局快捷键**：通过 `org.kde.KGlobalAccel` 注册全局快捷键，默认 `Meta+H` 单键切换录音。
- **手动录音模式**：保留前台交互录音模式，便于调试与验证链路。

## 工作流程

`audiov` 的处理链路如下：

1. **Capture**：监听 KDE 全局快捷键或手动触发，并采集麦克风音频（PipeWire/PulseAudio）。
2. **Inference**：调用内置 `whisper.cpp` 引擎完成语音转文本。
3. **Clipboard**：通过 `arboard` 将文本写入系统剪贴板。
4. **Inject**：通过外部粘贴命令，或通过 `fcitx5` addon 直接提交文本。
5. **Restore（可选）**：恢复用户原有剪贴板内容。

## 快速开始

> 当前项目仍在开发中（WIP），但已支持最小可用的前后台运行与基础自检。

### 依赖项

请先准备以下组件：

- Rust 工具链（`cargo`）
- `ydotoold`（若使用 `paste.mode = "command"`）
- `fcitx5`（若使用 `paste.mode = "fcitx5"`）
- KDE Plasma（若使用默认全局快捷键模式）

### 编译与运行

```bash
git clone https://github.com/yourusername/audiov.git
cd audiov
cargo build --release

# 默认运行 KDE 全局快捷键模式
./target/release/audiov --config ~/.config/audiov/config.toml

# 手动录音模式
./target/release/audiov --manual --config ~/.config/audiov/config.toml
```

### fcitx5 模式快速路径

如果你要绕开 `ydotool`/`uinput`，直接走 `fcitx5`：

```bash
./contrib/fcitx5-audiov/install-user.sh
./contrib/fcitx5-audiov/restart-fcitx5.sh
busctl --user introspect org.fcitx.Fcitx5 /org/freedesktop/Fcitx5/Audiov
```

然后把配置切到：

```toml
[paste]
mode = "fcitx5"
command = []
fcitx5_service = "org.fcitx.Fcitx5"
fcitx5_path = "/org/freedesktop/Fcitx5/Audiov"
fcitx5_interface = "org.fcitx.Fcitx5.Audiov1"
```

最后运行：

```bash
cargo run -- --config /home/white/Projects/audiov/config.local.toml --manual
```
### 启动参数

- 默认模式：连接 `org.kde.KGlobalAccel`，按一次快捷键开始录音，再按一次同一快捷键结束并转写。
- `--manual`：前台交互录音模式，按一次 Enter 开始录音，再按一次 Enter 结束并转写。
- `--config <path>`：指定配置文件路径。
- `--transcribe-wav <path>`：直接转写一个 16kHz / mono / PCM16 WAV 文件。

配置文件默认查找顺序：

1. `AUDIOV_CONFIG` 环境变量；
2. `~/.config/audiov/config.toml`（若存在）；
3. 若不存在则直接报错（需要显式提供配置文件）。

### KDE 全局快捷键

默认热键配置如下：

```toml
[hotkey]
enabled = true
shortcut = "Meta+H"
component_unique = "audiov"
component_friendly = "audiov"
action_unique = "toggle-recording"
action_friendly = "Toggle Recording"
```

启动后 `audiov` 会通过 session D-Bus 连接 `org.kde.KGlobalAccel`，把快捷键注册到 KDE 的全局快捷键系统中。

- 第一次按下快捷键：开始录音
- 第二次按下同一快捷键：停止录音，开始转写并粘贴

当前快捷键字符串支持常见组合键，例如 `Meta+H`、`Ctrl+Shift+F5`、`Alt+Space`。

## 语言识别（LID）接入决策（已确认）

当前已确定的接入策略如下：

- **自动语言识别**：每次按下快捷键触发的录音会话，先做一次语言识别。
- **会话级识别**：仅在每次录音会话开始阶段识别一次，避免分段识别带来的额外延迟。
- **推理参数联动**：默认只记录识别结果，不强制把检测语言传给 `whisper.cpp`，以适配中英混合语音；如有纯单语低延迟需求，可在配置中显式开启。
- **语言白名单**：首期只允许 `zh` / `en`，降低误判空间并控制时延。
- **配置文件化**：相关策略放入 TOML 配置（见 `config.example.toml`）。
- **性能偏好**：优先低延迟。
- **验收目标**：中英场景语言识别准确率 **>95%**。

### 建议的最小实现流程

1. 录音结束后，先对该段音频执行 LID。
2. 若识别结果在白名单内且置信度达标，则记录该结果供日志与后续策略使用。
3. 默认保持 Whisper 转写阶段自动识别；如需纯单语优化，可再显式把检测语言传给推理。
4. 在 debug 日志中记录：候选语言、分数、最终采用语言。


## 当前实现进展

已完成一个可运行的 LID 决策模块最小实现（Rust）：

- `src/config.rs`：TOML 配置解析与默认值（对应 `config.example.toml`）。
- `src/lid.rs`：按白名单 + 置信度阈值选择 Whisper 推理语言。
- `src/pipeline.rs`：会话级处理流程（一次录音会话内先做 LID，再把最终语言参数传给转写器）。
- `src/whisper_cpp.rs`：`whisper-rs` 库调用实现（直接调 whisper.cpp 库完成语言检测与带语言参数转写）。
- `src/main.rs`：最小演示入口，使用 `WhisperCppEngine` 串起配置加载、LID 决策与转写调用。

可通过 `cargo test` 验证配置解析、语言决策以及“检测结果是否真实传入转写器”的核心逻辑。

当前最小可用交互方式包括两种：

- KDE 全局快捷键模式：按一次快捷键开始录音，再按一次结束。
- 手动模式：按一次 Enter 开始录音，再按一次 Enter 结束。

两种模式在结束录音后都会执行转写，把文本写入剪贴板，并调用可配置命令向当前窗口发送粘贴按键。

Linux 下的剪贴板内容由当前拥有者托管。`audiov` 在写入后会保持一个后台持有者，直到下一次写入替换它，以避免文本在粘贴前失效。

若切换到 `paste.mode = "fcitx5"`，`audiov` 会在写入剪贴板后，通过 session D-Bus 调用一个 `fcitx5` addon 的 `CommitText` 方法，把识别结果直接提交给当前聚焦输入上下文。用户级安装、重启和 smoke test 见 [contrib/fcitx5-audiov/README.md](/home/white/Projects/audiov/contrib/fcitx5-audiov/README.md)。

录音模块已抽象为 `NativeRecorder`，支持 `auto` / `pipewire` / `pulseaudio` / `alsa` 四种后端选择，默认优先尝试 PipeWire，再回退 PulseAudio 与 ALSA。


### whisper.cpp 运行前准备

请确保本地 `whisper.cpp` 动态库可被 `whisper-rs` 正常加载，并准备好模型文件（与 `config.example.toml` 的 `[whisper_cpp]` 对应）。



### 远端 Whisper 配置

可通过配置文件启用远端转写：

```toml
[whisper]
backend = "remote"

[whisper_remote]
enabled = true
endpoint = "https://api.openai.com/v1/audio/transcriptions"
model = "whisper-1"
api_key = "<YOUR_API_KEY>"
timeout_secs = 60
```

当 `backend = "remote"`（或 `whisper_remote.enabled = true`）时，程序会调用远端 HTTP API；否则继续使用本地 `whisper.cpp`。

### 运行期可观测性与自检

程序启动时会输出基础依赖检查告警（模型文件、`DBUS_SESSION_BUS_ADDRESS`、`arecord`、粘贴命令），并在每次会话打印 LID 最终采用语言与原因，便于排障。
