# audiov

> 让语音输入像键盘操作一样直接。

`audiov`（Audio + V）是一个使用 Rust 编写的 Linux 全局语音输入守护进程。它坚持本地优先、无云依赖，并通过“剪贴板 + 虚拟输入设备”的方式，在不同桌面协议下稳定完成文本输入。

## 核心特性

- **本地离线识别**：基于 `whisper.cpp`，支持在本机 CPU/GPU 上完成语音转写，数据不离开设备。
- **稳定文本注入**：通过系统剪贴板与 `/dev/uinput`（或 `ydotool`）组合，减少中文与 Unicode 字符输入异常。
- **兼容 Wayland / X11**：避免依赖脆弱的图形层模拟，适配主流 Linux 桌面环境。
- **守护进程模式**：支持后台常驻与全局快捷键触发，按下录音、松开粘贴，流程简洁。

## 工作流程

`audiov` 的处理链路如下：

1. **Capture**：监听全局快捷键并采集麦克风音频（PipeWire/PulseAudio）。
2. **Inference**：调用内置 `whisper.cpp` 引擎完成语音转文本。
3. **Clipboard**：将文本写入系统剪贴板。
4. **Inject**：通过 `/dev/uinput`（或 `ydotool`）发送粘贴快捷键（如 `Ctrl+V` / `Ctrl+Shift+V`）。
5. **Restore（可选）**：恢复用户原有剪贴板内容。

## 快速开始

> 当前项目仍在开发中（WIP），但已支持最小可用的前后台运行与基础自检。

### 依赖项

请先准备以下组件：

- Rust 工具链（`cargo`）
- `ydotoold`（或自行配置 `uinput`/`udev` 权限）
- 剪贴板工具（例如 Wayland 下的 `wl-clipboard`）

### 编译与运行

```bash
git clone https://github.com/yourusername/audiov.git
cd audiov
cargo build --release

# 运行守护进程
./target/release/audiov --daemon --config ~/.config/audiov/config.toml
```
### 启动参数

- `--daemon`：以后台模式启动（会拉起一个 `--foreground` 子进程并返回）。
- `--foreground`：前台运行（默认）。
- `--config <path>`：指定配置文件路径。

配置文件默认查找顺序：

1. `AUDIOV_CONFIG` 环境变量；
2. `~/.config/audiov/config.toml`（若存在）；
3. 若不存在则直接报错（需要显式提供配置文件）。


## 语言识别（LID）接入决策（已确认）

当前已确定的接入策略如下：

- **自动语言识别**：每次按下快捷键触发的录音会话，先做一次语言识别。
- **会话级识别**：仅在每次录音会话开始阶段识别一次，避免分段识别带来的额外延迟。
- **推理参数联动**：识别到语言后，将该语言直接传给 `whisper.cpp` 作为推理语言参数，以提升速度与稳定性。
- **语言白名单**：首期只允许 `zh` / `en`，降低误判空间并控制时延。
- **配置文件化**：相关策略放入 TOML 配置（见 `config.example.toml`）。
- **性能偏好**：优先低延迟。
- **验收目标**：中英场景语言识别准确率 **>95%**。

### 建议的最小实现流程

1. 录音结束后，先对该段音频执行 LID。
2. 若识别结果在白名单内且置信度达标，则将其作为 Whisper 推理语言。
3. 否则回退到 `default_language`（默认 `zh`）。
4. 在 debug 日志中记录：候选语言、分数、最终采用语言。


## 当前实现进展

已完成一个可运行的 LID 决策模块最小实现（Rust）：

- `src/config.rs`：TOML 配置解析与默认值（对应 `config.example.toml`）。
- `src/lid.rs`：按白名单 + 置信度阈值选择 Whisper 推理语言。
- `src/pipeline.rs`：会话级处理流程（一次录音会话内先做 LID，再把最终语言参数传给转写器）。
- `src/whisper_cpp.rs`：`whisper-rs` 库调用实现（直接调 whisper.cpp 库完成语言检测与带语言参数转写）。
- `src/main.rs`：最小演示入口，使用 `WhisperCppEngine` 串起配置加载、LID 决策与转写调用。

可通过 `cargo test` 验证配置解析、语言决策以及“检测结果是否真实传入转写器”的核心逻辑。

新增了最小可用的全局按键按住说话流程：监听全局热键（默认 `Windows+H`）按下开始录音、松开结束录音，随后执行转写，把文本写入剪贴板，并调用可配置命令向当前窗口发送粘贴按键。

热键解析支持组合键，例如 `windows+h`、`ctrl+space`、`f8`。

录音模块已抽象为 `NativeRecorder`，支持 `auto` / `pipewire` / `pulseaudio` / `alsa` 四种后端选择，默认优先尝试 PipeWire，再回退 PulseAudio 与 ALSA。


### whisper.cpp 运行前准备

请确保本地 `whisper.cpp` 动态库可被 `whisper-rs` 正常加载，并准备好模型文件（与 `config.example.toml` 的 `[whisper_cpp]` 对应）。


### 运行期可观测性与自检

程序启动时会输出基础依赖检查告警（模型文件、`arecord`、剪贴板工具、粘贴命令），并在每次会话打印 LID 最终采用语言与原因，便于排障。
