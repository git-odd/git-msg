# git-msg

[![Crates.io](https://img.shields.io/crates/v/git-msg.svg)](https://crates.io/crates/git-msg)
[![License](https://img.shields.io/crates/l/git-msg.svg)](https://crates.io/crates/git-msg)

[English](README.md) | [中文](README.zh.md)

`git-msg` 是一个基于 Rust 开发的轻量级、本地优先（Local-First）的智能 Git 提交信息生成工具。它作为 Git 原生子命令（`git msg`）无缝集成到日常开发工作流中，自动分析暂存区或工作区代码差异（Diff），结合上下文生成规范的 Conventional Commits、Gitmoji 或简明风格提交信息。

---

## ✨ 核心特性

* **开箱即用（Zero-Config DX）**：默认配置直接适配本地 OpenAI 兼容端点（如 LM Studio / Ollama 的 `http://127.0.0.1:1234`），无需复杂初始设置。
* **本地与小模型专属优化**：针对 2B–9B 参数量模型（如 `qwen3.5-2b`）深度设计 Few-Shot Prompt，内置输出清洗器，自动剥离 `<think>` 推理标签、Markdown 代码围栏及前导多余提示词。
* **Git 原生子命令体验**：安装后在任意 Git 仓库目录下直接敲击 `git msg` 即可运行。
* **智能 Diff 处理管道**：
  * 暂存区为空时，自动检测工作区并通过 `git add -N` 捕获新增的未跟踪文件（Untracked Files）。
  * 自动过滤锁定文件与无意义改动（如 `Cargo.lock`、`package-lock.json`、压缩资源等）。
  * 单文件配额（150行）与全局上限（500行）两级截断保护，防止大文件耗尽上下文。
  * **敏感凭证脱敏**：在向大模型发送 Diff 前，自动正则脱敏常见 API Key 与 Token（`sk-...`、`ghp_...`、`AKIA...` 等）。
* **跨平台安全提交**：使用临时文件与 `git commit -F` 执行提交，彻底杜绝 Windows/Linux/macOS 下 Shell 传递多行文本与引号转义异常。
* **友好交互式终端 UX**：具备加载动画、边框高亮预览与快捷操作菜单（`Commit 确认提交`、`Edit 编辑器修改`、`Regenerate 重新生成`、`Abort 取消退出`）。

---

## 🚀 安装指南

### 方式一：通过 Cargo 安装（推荐）

```bash
cargo install git-msg
```

### 方式二：从 Git 仓库安装

```bash
cargo install --git https://github.com/git-odd/git-msg.git
```

### 方式三：从源码本地安装

```bash
cargo install --path .
```

安装完成后，即可在任意 Git 项目中使用 `git msg`。

---

## 📖 使用说明

### 标准提交流程

在修改完代码后，直接运行：

```bash
git msg
```

`git-msg` 将会自动：
1. 提取并过滤差异（若暂存区为空且开启自动暂存，将自动包含工作区改动）；
2. 调用大模型生成结构化提交信息；
3. 弹出高亮预览框与操作菜单：
   * **Commit**：确认无误，直接提交。
   * **Edit**：调起系统偏好编辑器（`git var GIT_EDITOR` / `$EDITOR`）进行修改，保存退出后以新内容提交。
   * **Regenerate**：重新调用模型生成。
   * **Abort**：取消并退出，不做任何 Git 写入。

### 命令行选项 (Flags)

```bash
# 跳过交互式确认，直接确认提交
git msg -y

# 仅向 stdout 打印生成的提交信息（不触发交互与 Git 写入）
git msg -d

# 临时指定模板类型（conventional, simple, gitmoji）
git msg -t gitmoji

# 临时覆盖使用的模型名称或端点地址
git msg -m qwen3.5-2b -e http://127.0.0.1:1234
```

### 配置子命令

```bash
# 使用系统编辑器打开全局配置文件
git msg config

# 仅查看全局配置文件的绝对路径
git msg config --show-path

# 在当前 Git 仓库根目录下初始化 .gitmsg.toml 模板
git msg init
```

---

## ⚙️ 配置系统

`git-msg` 按以下优先级加载与合并配置：
1. 命令行参数（CLI Flags）
2. 环境变量（`GIT_MSG_ENDPOINT`, `GIT_MSG_MODEL`, `GIT_MSG_API_KEY`, `OPENAI_API_KEY`）
3. 项目级配置（当前仓库根目录下的 `.gitmsg.toml`）
4. 用户全局配置（`~/.config/git-msg/config.toml` 或 Windows `%APPDATA%\git-msg\config.toml`）
5. 内置默认值

> 🔒 **安全提示**：对于云端 API Key（如 OpenAI / DeepSeek），建议保存在用户全局配置（`git msg config`）或环境变量 `$OPENAI_API_KEY` 中，避免将密钥误提交到 Git 仓库。

### 配置文件结构示例 (`.gitmsg.toml` / `config.toml`)

```toml
# git-msg configuration

[provider]
endpoint = "http://127.0.0.1:1234"
# 本地模型保持 "not-needed"；云端模型建议通过 $OPENAI_API_KEY 或全局配置 (`git msg config`) 设置
api_key = "not-needed"
model = "qwen3.5-2b"
timeout_seconds = 30
temperature = 0.2

[behavior]
template = "conventional"        # Options: conventional | simple | gitmoji
language = "en-US"               # Options: en-US | zh-CN
auto_stage_if_empty = true       # 暂存区为空时是否自动暂存工作区所有更改
max_diff_lines = 500             # 全局 Diff 总行数截断上限
max_file_diff_lines = 150        # 单文件最大 Diff 行数

[diff_filter]
ignore_files = [
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "*.min.js",
    "*.min.css",
    "*.svg",
    "*.lock"
]

# 可选：自定义 Prompt，若配置则完全覆盖内置 template
# 可用变量: {diff}, {recent_commits}, {language}
# custom_prompt = """
# 你是一个提交信息生成器，根据以下 Diff 生成中文提交信息：
# {diff}
# """
```

---

## 📄 许可证

本项目基于 MIT OR Apache-2.0 协议开源。
