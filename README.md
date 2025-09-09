# learn_egui 计数器 Demo

一个使用 winit + egui + wgpu 的最小计数器应用，帮助回顾 Rust 语法并了解事件循环与即时模式 UI。

## 运行

Windows (cmd):

```
set RUST_LOG=info && cargo run
```

首次构建会下载依赖，时间较久属正常。

若看到 `Blocking waiting for file lock on package cache`，说明有其它 cargo 进程占用缓存，请关闭其它 `cargo build/run` 或稍候再试。

## 操作

- 点击 +1 / -1 / 重置 按钮修改计数。
- 调整窗口大小查看自适应渲染。

## 结构

- `src/main.rs`：
  - `CounterApp`：应用状态与 UI 构建（egui 立即模式）。
  - `GfxState`：wgpu 初始化与 surface 配置。
  - `AppState`：winit 0.29 ApplicationHandler 事件驱动框架。
- 渲染路径：egui -> tessellate -> egui_wgpu::Renderer -> wgpu 命令提交 -> 呈现。

## 参考

- winit 0.29 ApplicationHandler API
- egui/egui-winit/egui-wgpu 0.28
- wgpu 0.20
