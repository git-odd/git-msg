# `git-msg` 架构与实现规范 (Technical Specification)

## 1. 设计目标与原则
* **Zero-Config DX**：未配置时敲击 `git msg` 即可配合本地默认端点（如 LMStudio / 本地 LLM 服务的 `http://127.0.0.1:1234`）直接运行。
* **Local-First**：针对 2B–9B 本地小参数及量化模型进行 Prompt 约束与输出清洗（Sanitization），同时兼容各类云端 OpenAI 格式端点（DeepSeek, Qwen 等）。
* **Git-Native**：遵循 Git 子命令机制（二进制命名为 `git-msg`），深度继承 Git 配置（如 `core.editor`），保留完整的 Git 提交生命周期与系统编辑器控制权。

---

## 2. CLI 命令与参数规范

### 2.1 主命令与子命令
* `git msg`：默认执行流程（分析 Diff -> 生成 Message -> 交互式提交）。
* `git msg config`：使用系统编辑器打开全局配置文件（若不存在则自动以默认模板创建）。
  * 选项：`--show-path`（仅打印配置文件绝对路径，不调起编辑器）。
* `git msg init`：在当前 Git 仓库根目录下生成一份 `.gitmsg.toml` 模板文件。

### 2.2 Flags (应用于 `git msg`)
* `-y, --yes`：跳过交互，生成后自动确认提交（若使用未暂存的修改，会自动触发 `git add -A`）。
* `-d, --dry-run`：仅向 `stdout` 打印生成的 Message，不产生交互，不执行任何 Git 写入操作。
* `-t, --template <NAME>`：临时指定模板类型（`conventional` | `simple` | `gitmoji`）。
* `-m, --model <MODEL>`：临时覆盖使用的模型名称。
* `-e, --endpoint <URL>`：临时覆盖请求的端点地址。

---

## 3. 配置系统设计 (Configuration)

### 3.1 查找优先级
1. 命令行参数 (CLI Flags)
2. 环境变量 (`GIT_MSG_ENDPOINT`, `GIT_MSG_MODEL`, `GIT_MSG_API_KEY`)
3. 项目级配置（当前 Git 仓库根目录下的 `.gitmsg.toml`）
4. 用户全局配置（`~/.config/git-msg/config.toml` 或 Windows `%APPDATA%\git-msg\config.toml`）
5. 内置默认值（Hardcoded Defaults）

### 3.2 配置文件结构 Schema (`config.toml` / `.gitmsg.toml`)

```toml
[provider]
# 支持输入基础 URL（如 http://127.0.0.1:1234）或带版本路径（如 http://localhost:1234/v1）
endpoint = "http://127.0.0.1:1234"
api_key = "not-needed"
model = "qwen3.5-2b"
timeout_seconds = 30
temperature = 0.2

[behavior]
# 内置模板: "conventional" | "simple" | "gitmoji"
template = "conventional"
language = "en-US"               # "en-US" | "zh-CN"
auto_stage_if_empty = true       # 暂存区为空时是否自动暂存工作区所有更改
max_diff_lines = 500             # 全局 Diff 总行数截断上限
max_file_diff_lines = 150        # 单文件最大 Diff 行数（避免单大文件耗尽配额）

[diff_filter]
# 忽略不参与 Diff 提取的文件模式 (Glob 匹配)
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

## 4. Diff 收集与预处理管道 (Pipeline)

### 4.1 收集与暂存探测
```
[git diff --cached] 
       │ 
 (为空?) ──Yes──> [检测工作区已跟踪与未跟踪文件]
       │                 │
       │          (检查 git status --porcelain，并使用 git add -N 捕获未跟踪文件)
       │                 │
      No          (工作区仍为空?) ──Yes──> 打印 "No changes detected." -> 退出
       │                 │
       │          (存在改动且 auto_stage_if_empty = true)
       │                 │
       │          标记 need_stage = true, 提取完整工作区 diff
       │                 │
       └────────┬────────┘
                ▼
      [Diff Filter 过滤] ──(剔除 ignore_files 中的匹配文件、忽略二进制文件)
                ▼
      [Diff Truncate 截断保护]
        1. 单文件超出 max_file_diff_lines 时截断并在文件尾部追加 "[... X lines truncated ...]"
        2. 全局行数超出 max_diff_lines 时截断剩余文件并在末尾追加全局截断说明
                ▼
      [Context 提取] ──(获取 git log -n 3 --oneline 作为风格参考；初次提交无历史时平滑忽略)
```

---

## 5. Prompt 与模型交互规范

### 5.1 默认 System Prompt（以 Conventional Commits 为例）
```text
You are an expert Git commit message generator.
Analyze the provided code diff and recent commits, then output a clean and concise commit message.

Strict Rules:
1. Output MUST contain ONLY the commit message. No explanations, no markdown blocks, no prefix text.
2. Follow Conventional Commits format: <type>(<optional-scope>): <subject>
3. Use language: {language}.
4. Keep the subject line under 72 characters, imperative mood, no ending period.
5. If necessary, provide a brief body separated by an empty line using bullet points.
```

### 5.2 请求 Payload 与 URL 规范化
1. **URL 规范化**：
   * 输入 `http://127.0.0.1:1234` $\rightarrow$ 规范化为 `http://127.0.0.1:1234/v1/chat/completions`。
   * 输入 `http://localhost:1234/v1` $\rightarrow$ 规范化为 `http://localhost:1234/v1/chat/completions`。
   * 输入已包含完整路径时直接保留。
2. **Payload 结构**：
   * `POST .../chat/completions`
   * `messages`: `system` 角色说明 + `user` 输入（包含过滤后的 Diff + 历史提交记录）。
   * `temperature`: `0.2`。

### 5.3 输出清洗器 (Sanitizer)
针对小参数模型及推理模型可能附带的多余输出，执行严格清洗流程：
1. **剥离推理标签**：去除 `<think>[\s\S]*?</think>`。
2. **剥离 Markdown 围栏**：去除首尾 ` ```markdown ` / ` ```git ` / ` ``` ` 标记。
3. **剥离前导提示语**：去除常见的 `Here is the commit message:`、`Commit message:`、`提交信息：` 等前导行。
4. **格式规范化**：修剪首尾多余空白换行，确保输出为纯正的 Git Commit Message 文本。

---

## 6. 交互式生命周期 (Terminal UX)

1. **执行开始**：
   * 启动 `indicatif` 动画（提示 `Analyzing diff & generating commit message...`）。
2. **获取响应**：
   * 停止动画，清洗后在终端以清晰边框高亮输出生成的 Commit Message。
3. **分流处理**：
   * 若含 `--yes`：若 `need_stage == true` 则先执行 `git add -A`，随后直接提交。
   * 默认交互：使用 `inquire::Select` 弹出单选菜单：
     * `Commit`: 确认提交。
     * `Edit`: 调用系统编辑器打开临时文件进行人工修改，保存后以修改内容提交。
     * `Regenerate`: 重新调用模型生成。
     * `Abort`: 取消并退出。
4. **编辑器与 Commit 执行安全机制**：
   * **编辑器解析优先级**：`git var GIT_EDITOR` $\rightarrow$ `$GIT_EDITOR` $\rightarrow$ `$VISUAL` $\rightarrow$ `$EDITOR` $\rightarrow$ Windows 下回退到 `notepad`，Unix 下回退到 `vi`。
   * **安全 Commit**：将最终 Message 写入临时文件，执行 `git commit -F <temp_file>`，完全避免 Shell 传递多行文本和引号时的转义问题。

---

## 7. Rust 模块结构与依赖选型

### 7.1 Cargo 依赖项
```toml
[dependencies]
clap = { version = "4.4", features = ["derive", "env"] }
reqwest = { version = "0.11", features = ["json", "blocking"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
inquire = "0.7"
indicatif = "0.17"
colored = "2.1"
glob = "0.3"
tempfile = "3.10"
dirs = "5.0"
regex = "1.10"
anyhow = "1.0"
```

### 7.2 模块划分
```
src/
├── main.rs          # 程序入口，主流程调度与命令分发
├── cli.rs           # Clap 命令行结构定义
├── config.rs        # 多级配置加载、合并、默认值与路径解析
├── git.rs           # 封装 git diff, git log, git status, git commit -F, 编辑器调用
├── filter.rs        # Diff 忽略文件过滤、单文件/全局行数截断
├── llm.rs           # HTTP 客户端封装（URL规范化、OpenAI协议交互）
├── sanitizer.rs     # 剥离 think 标签、Markdown 块、前导提示词
├── prompt.rs        # 模板管理与变量填充
└── ui.rs            # Inquire 菜单与 Indicatif 动画
```

---

## 8. 异常处理标准 (Error Handling)

1. **LLM 连接失败（`ECONNREFUSED` / Timeout）**：
   * 拦截底层网络错误，输出结构化提示：
     ```text
     Error: Failed to connect to LLM endpoint at [URL].
     → Is your local LLM (e.g. LMStudio / Ollama) running?
     → Use 'git msg config' to check or modify your configuration.
     ```
2. **Git Commit 失败（如 Pre-commit hook 拦截、GPG 签名错误）**：
   * 透传 Git 原始 stderr 输出。
   * 在终端保留本次生成的 Message 文本或提示临时文件位置，防止用户编辑内容丢失。
3. **初次提交（无 HEAD 分支 / 无历史 commit）**：
   * `git log` 获取 context 失败时平滑降级（忽略历史 commit 上下文），不中断主提交流程。