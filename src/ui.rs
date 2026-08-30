use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Select;
use std::fmt;
use std::time::Duration;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserAction {
    Commit,
    Edit,
    Regenerate,
    Abort,
}

pub struct ActionItem {
    pub action: UserAction,
    pub language: String,
}

impl fmt::Display for ActionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.language == "zh-CN" {
            match self.action {
                UserAction::Commit => write!(
                    f,
                    "{}",
                    "提交 (Commit)       - 确认并执行提交".green().bold()
                ),
                UserAction::Edit => {
                    write!(f, "{}", "编辑 (Edit)         - 打开编辑器修改后提交".cyan())
                }
                UserAction::Regenerate => {
                    write!(f, "{}", "重新生成 (Regenerate) - 重新调用模型生成".yellow())
                }
                UserAction::Abort => write!(f, "{}", "取消 (Abort)        - 取消并退出".red()),
            }
        } else {
            match self.action {
                UserAction::Commit => write!(
                    f,
                    "{}",
                    "Commit       - Confirm and execute commit".green().bold()
                ),
                UserAction::Edit => {
                    write!(f, "{}", "Edit         - Open in editor and commit".cyan())
                }
                UserAction::Regenerate => write!(
                    f,
                    "{}",
                    "Regenerate   - Re-generate commit message".yellow()
                ),
                UserAction::Abort => write!(f, "{}", "Abort        - Cancel and exit".red()),
            }
        }
    }
}

pub struct Ui;

impl Ui {
    /// 创建并启动加载 Spinner
    pub fn create_spinner(message: &'static str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner()),
        );
        pb.set_message(message);
        pb
    }

    /// 在终端输出边框对齐的高亮 Commit Message 预览框（支持 CJK 全角字符宽度）
    pub fn render_commit_message_box(msg: &str, language: &str) {
        let lines: Vec<&str> = msg.lines().collect();

        let title = if language == "zh-CN" {
            " 建议的提交信息 "
        } else {
            " Proposed Commit Message "
        };

        let title_width = UnicodeWidthStr::width(title);

        // 计算所有行中最大的终端显示宽度
        let max_content_width = lines
            .iter()
            .map(|l| UnicodeWidthStr::width(*l))
            .max()
            .unwrap_or(36)
            .max(title_width + 4)
            .max(36);

        // 内部宽度（含两侧各 2 空格内边距）
        let inner_width = max_content_width + 4;

        // 顶部边框
        let top_filler_len = inner_width.saturating_sub(title_width + 2);
        let top_border = format!(
            "┌─{}{}{}",
            title.bold().cyan(),
            "─".repeat(top_filler_len),
            "┐"
        );

        println!();
        println!("{}", top_border.bright_black());

        for line in &lines {
            let line_w = UnicodeWidthStr::width(*line);
            let padding_len = max_content_width.saturating_sub(line_w);
            let padding = " ".repeat(padding_len);

            println!(
                "{}  {}  {}{}",
                "│".bright_black(),
                line.bold().white(),
                padding,
                "│".bright_black()
            );
        }

        let bottom_border = format!("└{}┘", "─".repeat(inner_width));
        println!("{}", bottom_border.bright_black());
        println!();
    }

    /// 弹出交互选择菜单
    pub fn prompt_action(language: &str) -> anyhow::Result<UserAction> {
        let options = vec![
            ActionItem {
                action: UserAction::Commit,
                language: language.to_string(),
            },
            ActionItem {
                action: UserAction::Edit,
                language: language.to_string(),
            },
            ActionItem {
                action: UserAction::Regenerate,
                language: language.to_string(),
            },
            ActionItem {
                action: UserAction::Abort,
                language: language.to_string(),
            },
        ];

        let prompt_text = if language == "zh-CN" {
            "请选择操作："
        } else {
            "Choose an action:"
        };

        let help_text = if language == "zh-CN" {
            "↑/↓ 切换选项，Enter 确认，Esc/Ctrl+C 取消"
        } else {
            "↑/↓ to navigate, Enter to select, Esc/Ctrl+C to abort"
        };

        let ans = Select::new(prompt_text, options)
            .with_help_message(help_text)
            .prompt()?;

        Ok(ans.action)
    }
}
