<div align="center">

# 💬 git-msg

**AI-powered Git commit message generator, local-first and lightweight.**

[![Organization](https://img.shields.io/badge/Org-git--odd-blue?style=flat-square&logo=github)](https://github.com/git-odd)
[![Suite](https://img.shields.io/badge/Suite-git--odd%20Ecosystem-purple?style=flat-square&logo=git)](https://github.com/git-odd)
[![Crates.io](https://img.shields.io/crates/v/git-msg.svg?style=flat-square)](https://crates.io/crates/git-msg)
[![License](https://img.shields.io/badge/License-MIT%20%2F%20Apache--2.0-orange?style=flat-square)](LICENSE-MIT)

[English](README.md) | [简体中文](README_zh.md)

</div>

> Part of the [**`git-odd`**](https://github.com/git-odd) suite — *solving odd Git problems in odd ways.*

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
  * **Secret Redaction**: Automatically redacts API keys and access tokens (`sk-...`, `ghp_...`, `AKIA...`) in diffs before sending to LLMs.
* **Safe & Robust Commits**: Uses temporary files with `git commit -F` to eliminate quote escaping and newline corruption across Windows/Linux/macOS shells.
* **Interactive Terminal UX**: Rich CLI experience with spinners, styled commit preview boxes, and quick action menus (`Commit`, `Edit`, `Regenerate`, `Abort`).

---

## 🚀 Installation

### Via Cargo (Recommended)

```bash
cargo install git-msg
```

### From Git Repository

```bash
cargo install --git https://github.com/git-odd/git-msg.git
```

### From Local Source

```bash
cargo install --path .
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

> 🔒 **Security Notice**: For cloud API keys (e.g. OpenAI / DeepSeek), store them in your global configuration (`git msg config`) or `$OPENAI_API_KEY` to avoid committing secrets into Git repositories.

### Example `.gitmsg.toml`

```toml
# git-msg configuration

[provider]
endpoint = "http://127.0.0.1:1234"
# Set to "not-needed" for local models. For cloud APIs, use $OPENAI_API_KEY or global config (`git msg config`)
api_key = "not-needed"
model = "qwen3.5-2b"
timeout_seconds = 30
temperature = 0.2

[behavior]
template = "conventional"        # Options: conventional | simple | gitmoji
language = "en-US"               # Options: en-US | zh-CN
auto_stage_if_empty = true       # Auto-stage working tree if staging area is empty
max_diff_lines = 500             # Global diff line limit for LLM context
max_file_diff_lines = 150        # Per-file diff line quota to avoid single-file saturation

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

# Optional: Override built-in templates with your custom prompt
# Variables: {diff}, {recent_commits}, {language}
# custom_prompt = """
# You are a commit generator. Generate a commit message based on:
# {diff}
# """
---

## 📄 License

Dual-licensed under either of:
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

