# Chess CLI (中国象棋命令行版)

![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-edition%202024-orange)
![Platform](https://img.shields.io/badge/platform-windows%20|%20macos%20|%20linux-lightgrey)

**Chess CLI** 是一个基于 Rust 编写的高性能、跨平台中国象棋（Xiangqi）命令行界面（TUI）。它实现了 UCI (Universal Chess Interface) 协议，允许用户加载并与强大的象棋引擎（如皮卡鱼 Pikafish）进行对弈或分析。

本项目致力于在终端环境中提供接近原生 GUI 的流畅体验，支持多核引擎并行计算、实时思考分析、悔棋/重做以及 FEN 局面的保存与加载。

## ✨ 特性 (Features)

*   **跨平台支持**: 完美运行于 Windows, macOS, 和 Linux。
*   **UCI 协议支持**: 兼容主流 UCI 象棋引擎（推荐使用 [Pikafish](https://www.pikafish.com/)）。
*   **终端用户界面 (TUI)**: 基于 `ratatui` 构建的现代化终端界面，支持鼠标操作（部分终端）和键盘导航。
*   **实时设置与控制**:
    *   **动态配置**: 在 TUI 中实时调整玩家执色、切换引擎、设置 MultiPV (多路分析) 和难度等级。
    *   **流程控制**: 支持游戏暂停（返回主菜单）和继续游戏，允许中途调整引擎参数。
*   **实时分析**: 显示引擎的思考深度、分数、NPS (Nodes Per Second) 和主要变例 (PV)。
*   **完整游戏功能**:
    *   悔棋 (Undo) / 重做 (Redo)
    *   保存 (Save) / 加载 (Load) 游戏局面 (FEN 格式)
    *   合法着法提示与验证 (防止自杀、送将等违规走法)
*   **高性能**: 利用 Rust 的 `tokio` 异步运行时，确保界面响应流畅，同时高效处理引擎通信。

## 🚀 安装 (Installation)

### 方式一：源码编译 (推荐)

确保你已安装 [Rust 工具链](https://www.rust-lang.org/tools/install)。

```bash
git clone https://github.com/pluto/chess-cli.git
cd chess-cli
cargo build --release
```

编译完成后，可执行文件位于 `target/release/chess-cli`。

### 方式二：跨平台编译

如果你需要为 Linux 编译 (在 macOS/Windows 上)：

```bash
cargo install cross
cross build --target x86_64-unknown-linux-gnu --release
```

## ⚙️ 配置 (Configuration)

在运行之前，你需要配置象棋引擎。

1.  下载 UCI 引擎（例如 [皮卡鱼 Pikafish](https://www.pikafish.com/)）。
2.  在可执行文件同级目录（或 `~/.config/chess-cli/`）创建 `engines.toml` 文件。

`engines.toml` 示例：

```toml
[pikafish]
path = "./pikafish-nnue-avx2"  # 引擎可执行文件的路径
# options 字段用于传递 UCI 选项
# options = { Threads = "4", Hash = "256" }
```

## 🎮 使用说明 (Usage)

运行程序：

```bash
./chess-cli
```

### 快捷键 (Controls)

| 按键 | 功能 |
| :--- | :--- |
| `n` | **开始新游戏** (默认执红，引擎执黑) |
| `c` | **继续游戏** (从主菜单返回游戏) |
| `s` | **进入设置** (选择红黑/引擎/难度/MultiPV) |
| `h` | **查看帮助** |
| `q` / `Esc` | **返回主菜单** (游戏中) / 退出程序 (主菜单) |
| `↑` `↓` `←` `→` | **移动光标** (支持翻转视角后的直觉操作) |
| `Enter` / `Space` | **选择 / 移动棋子** |
| `u` | **悔棋** (Undo) |
| `r` | **重做** (Redo) |
| `s` (游戏中) | **保存游戏** (保存为 saved_game.fen) |
| `l` (游戏中) | **加载游戏** (从 saved_game.fen 加载) |

## 🏗️ 架构 (Architecture)

本项目采用清晰的分层架构：

*   **`src/tui`**: 用户界面层。使用 `ratatui` 渲染，处理用户输入和事件循环。包含 `App` (应用状态), `UiState` (UI状态), `View` (视图路由)。
*   **`src/engine`**: 引擎交互层。基于 Actor 模型 (`tokio::spawn`) 管理引擎进程，通过管道 (Stdio) 进行 UCI 协议通信。支持动态参数调整。
*   **`src/game`**: 游戏逻辑层。负责棋盘状态管理 (`GameState`)、合法着法生成 (优化版)、FEN 解析与生成、中国象棋规则判定（包括将军、送将检测）。

## 📝 待办事项 (TODO)

- [x] 支持在 UI 中动态切换引擎
- [x] 支持游戏暂停与继续
- [x] 优化配置选项 (MultiPV, 难度)
- [ ] 增加对局时钟与时间控制
- [ ] 支持加载开局库 (.obk)

## 📄 许可证 (License)

本项目采用 [GPL-3.0](LICENSE) 协议开源。
