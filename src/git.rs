use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use anyhow::{bail, Context, Result};
use tempfile::NamedTempFile;

pub struct GitManager;

impl GitManager {
    /// 获取当前 Git 仓库的根目录路径
    pub fn get_repo_root() -> Result<PathBuf> {
        let output = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .context("Failed to execute 'git rev-parse --show-toplevel'. Is git installed?")?;

        if !output.status.success() {
            bail!("Not inside a Git repository.");
        }

        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path_str))
    }

    /// 收集 Diff：优先提取暂存区；若暂存区为空且允许 auto_stage，则探测工作区改动并包含未跟踪文件
    pub fn collect_diff(auto_stage_if_empty: bool) -> Result<(String, bool)> {
        // 1. 检查暂存区
        let staged_diff = Self::run_git_cmd(&["diff", "--cached"])?;
        if !staged_diff.trim().is_empty() {
            return Ok((staged_diff, false));
        }

        // 2. 暂存区为空，检查工作区是否有任何修改或未跟踪文件
        let status = Self::run_git_cmd(&["status", "--porcelain"])?;
        if status.trim().is_empty() {
            return Ok((String::new(), false));
        }

        // 3. 如果不需要自动暂存，仅返回已跟踪文件的工作区 diff
        if !auto_stage_if_empty {
            let working_diff = Self::run_git_cmd(&["diff"])?;
            return Ok((working_diff, false));
        }

        // 4. 处理未跟踪文件 (通过 git add -N 临时纳入 diff 追踪)
        for line in status.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("?? ") {
                let file_path = &trimmed[3..].trim();
                let _ = Command::new("git")
                    .args(["add", "-N", file_path])
                    .output();
            }
        }

        let working_diff = Self::run_git_cmd(&["diff"])?;
        Ok((working_diff, true))
    }

    /// 执行 git add -A
    pub fn stage_all() -> Result<()> {
        let output = Command::new("git")
            .args(["add", "-A"])
            .output()
            .context("Failed to execute 'git add -A'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("'git add -A' failed:\n{}", stderr);
        }

        Ok(())
    }

    /// 获取最近 N 条提交简短记录，用于大模型参考风格（空仓库时平滑降级返回空列表）
    pub fn get_recent_commits(count: usize) -> Vec<String> {
        let count_arg = format!("-n{}", count);
        let output = Command::new("git")
            .args(["log", &count_arg, "--oneline"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                return text
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }

        Vec::new()
    }

    /// 使用临时文件和 -F 参数安全提交，彻底规避 Shell 换行与引号转义异常
    pub fn commit_with_file(message: &str) -> Result<()> {
        let mut temp_file = NamedTempFile::new().context("Failed to create temporary file for commit message")?;
        temp_file
            .write_all(message.as_bytes())
            .context("Failed to write commit message to temporary file")?;
        temp_file.flush().context("Failed to flush temporary file")?;

        let temp_path = temp_file.path();

        let output = Command::new("git")
            .args(["commit", "-F", temp_path.to_str().unwrap_or_default()])
            .output()
            .context("Failed to execute 'git commit -F'")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            bail!("Git commit failed:\n{}{}", stdout, stderr);
        }

        Ok(())
    }

    /// 调用系统编辑器编辑提交信息
    pub fn open_editor_for_message(initial_content: &str) -> Result<String> {
        let mut temp_file = NamedTempFile::new().context("Failed to create temporary file for editing")?;
        temp_file
            .write_all(initial_content.as_bytes())
            .context("Failed to write initial content to temp file")?;
        temp_file.flush()?;

        let temp_path = temp_file.path().to_path_buf();
        let editor = Self::resolve_editor();

        let status = Self::launch_editor(&editor, &temp_path)?;
        if !status.success() {
            bail!("Editor exited with non-zero status.");
        }

        let edited_content = fs::read_to_string(&temp_path)
            .context("Failed to read edited message from temporary file")?;

        Ok(edited_content)
    }

    /// 解析编辑器调用链：git var GIT_EDITOR -> $GIT_EDITOR -> $VISUAL -> $EDITOR -> 系统默认
    fn resolve_editor() -> String {
        // 1. git var GIT_EDITOR
        if let Ok(output) = Command::new("git").args(["var", "GIT_EDITOR"]).output() {
            if output.status.success() {
                let ed = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !ed.is_empty() {
                    return ed;
                }
            }
        }

        // 2. 环境变量
        if let Ok(ed) = env::var("GIT_EDITOR") {
            if !ed.trim().is_empty() {
                return ed;
            }
        }
        if let Ok(ed) = env::var("VISUAL") {
            if !ed.trim().is_empty() {
                return ed;
            }
        }
        if let Ok(ed) = env::var("EDITOR") {
            if !ed.trim().is_empty() {
                return ed;
            }
        }

        // 3. 平台默认
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    }

    fn launch_editor(editor_cmd: &str, file_path: &Path) -> Result<std::process::ExitStatus> {
        #[cfg(windows)]
        {
            // Windows 下如果 editor 命令包含参数（例如 code --wait 或 notepad）
            let status = Command::new("cmd")
                .args(["/C", editor_cmd, file_path.to_str().unwrap_or_default()])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .with_context(|| format!("Failed to launch editor '{}'", editor_cmd))?;
            Ok(status)
        }

        #[cfg(not(windows))]
        {
            let status = Command::new("sh")
                .args(["-c", &format!("{} \"$1\"", editor_cmd), "--", file_path.to_str().unwrap_or_default()])
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .with_context(|| format!("Failed to launch editor '{}'", editor_cmd))?;
            Ok(status)
        }
    }

    fn run_git_cmd(args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .output()
            .with_context(|| format!("Failed to run 'git {}'", args.join(" ")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("'git {}' failed:\n{}", args.join(" "), stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}
