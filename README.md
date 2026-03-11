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

> 当前项目仍在开发中（WIP）。

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
./target/release/audiov --daemon
```
