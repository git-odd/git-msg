# git-msg

[English](README.md) | [中文](README.zh.md)

`git-msg` is a smart, lightweight, and local-first AI Git commit message generator built in Rust. It seamlessly integrates as a native Git subcommand (`git msg`), analyzes your staged or working tree diffs, and automatically generates high-quality commit messages conforming to Conventional Commits, Gitmoji, or custom styles.

---

## ✨ Key Features

* **Zero-Config DX**: Works out of the box with local OpenAI-compatible endpoints (e.g. LM Studio / Ollama on `http://127.0.0.1:1234`).
* **Local-First & Small-Model Optimized**: Specially crafted Few-Shot prompts and output sanitizers for 2B–9B parameter models (e.g. `qwen3.5-2b`), stripping `<think>` tags and markdown code blocks automatically.
* **Git-Native Subcommand**: Invoked simply via `git msg` anywhere in your Git workspace.
* **Smart Diff Pipeline**:
  * Automatically detects untracked new files via `git add -N` when the staging area is empty.
  * Filters out noise files (such as `Cargo.lock`, `package-lock.json`, minified assets).
  * Two-tier truncation protection (per-file quota and global line limits).
* **Safe & Robust Commits**: Uses temporary files with `git commit -F` to eliminate quote escaping and newline corruption across Windows/Linux/macOS shells.
* **Interactive Terminal UX**: Rich CLI experience with spinners, styled commit preview boxes, and quick action menus (`Commit`, `Edit`, `Regenerate`, `Abort`).

---

## 🚀 Installation

Ensure you have Rust and Cargo installed:

```bash
cargo install --path .
```

Or install from crates.io / git repository:

```bash
cargo install --git https://github.com/git-odd/git-msg.git
```

Once installed, `git msg` is ready to use in any Git repository.

---

## 📖 Usage

### Standard Workflow

Run `git msg` in your repository after making code changes:

```bash
git msg
```

`git-msg` will:
1. Extract and filter changes from the staging area (or automatically inspect unstaged changes if the staging area is empty).
2. Generate a structured commit message.
3. Display a preview box and interactive menu:
   * **Commit**: Confirm and execute the commit.
   * **Edit**: Open the message in your preferred editor (`git var GIT_EDITOR` / `$EDITOR`), then commit upon saving.
   * **Regenerate**: Ask the model for a fresh commit message.
   * **Abort**: Cancel without modifying the repository.

### Command Options

```bash
# Auto-confirm and commit without interactive prompt
git msg -y

# Dry-run: print the generated message to stdout only (no git writes)
git msg -d

# Choose a specific template (conventional, simple, gitmoji)
git msg -t gitmoji

# Override model or endpoint on the fly
git msg -m qwen3.5-2b -e http://127.0.0.1:1234
```

### Configuration Subcommands

```bash
# Open global configuration in your default editor
git msg config

# Show the absolute path of the global configuration file
git msg config --show-path

# Initialize a project-level .gitmsg.toml in the current repository
git msg init
```

---

## ⚙️ Configuration

`git-msg` resolves configuration in the following order of priority:
1. CLI Flags (`-e`, `-m`, `-t`)
2. Environment Variables (`GIT_MSG_ENDPOINT`, `GIT_MSG_MODEL`, `GIT_MSG_API_KEY`, `OPENAI_API_KEY`)
3. Project Configuration (`.gitmsg.toml` in repository root)
4. Global Configuration (`~/.config/git-msg/config.toml` or `%APPDATA%\git-msg\config.toml`)
5. Built-in Defaults

### Example `.gitmsg.toml`

```toml
[provider]
# OpenAI-compatible API endpoint (base URL or with /v1 path)
endpoint = "http://127.0.0.1:1234"
api_key = "not-needed"
model = "qwen3.5-2b"
timeout_seconds = 30
temperature = 0.2

[behavior]
# Built-in templates: "conventional" | "simple" | "gitmoji"
template = "conventional"
language = "zh-CN"               # "zh-CN" | "en-US"
auto_stage_if_empty = true       # Auto-stage working changes if index is empty
max_diff_lines = 500             # Global maximum diff lines limit
max_file_diff_lines = 150        # Per-file maximum diff lines limit

[diff_filter]
# Glob patterns to exclude from diff analysis
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

# Optional: Custom prompt overriding default templates
# Available variables: {diff}, {recent_commits}, {language}
# custom_prompt = """
# You are a commit generator. Generate a commit message based on:
# {diff}
# """
```

---

## 📄 License

MIT OR Apache-2.0
