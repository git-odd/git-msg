use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Select;
use std::fmt;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserAction {
    Commit,
    Edit,
    Regenerate,
    Abort,
}

impl fmt::Display for UserAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserAction::Commit => write!(f, "{}", "Commit       (确认并执行提交)".green().bold()),
            UserAction::Edit => write!(f, "{}", "Edit         (打开编辑器修改后提交)".cyan()),
            UserAction::Regenerate => write!(f, "{}", "Regenerate   (重新调用模型生成)".yellow()),
            UserAction::Abort => write!(f, "{}", "Abort        (取消并退出)".red()),
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

    /// 在终端输出边框高亮的 Commit Message
    pub fn render_commit_message_box(msg: &str) {
        let lines: Vec<&str> = msg.lines().collect();
        let max_len = lines.iter().map(|l| l.chars().count()).max().unwrap_or(40).max(40);
        let border_len = max_len + 4;

        let title = " Proposed Commit Message ";
        let top_border_rest = if border_len > title.len() + 3 {
            "─".repeat(border_len - title.len() - 3)
        } else {
            "─".repeat(2)
        };

        println!();
        println!(
            "{}",
            format!("┌─{}{}{}", title.bold().cyan(), top_border_rest, "┐").bright_black()
        );

        for line in &lines {
            let padding = " ".repeat(max_len.saturating_sub(line.chars().count()));
            println!(
                "{}  {}  {}{}",
                "│".bright_black(),
                line.bold().white(),
                padding,
                "│".bright_black()
            );
        }

        println!(
            "{}",
            format!("└{}┘", "─".repeat(border_len - 2)).bright_black()
        );
        println!();
    }

    /// 弹出交互选择菜单
    pub fn prompt_action() -> anyhow::Result<UserAction> {
        let options = vec![
            UserAction::Commit,
            UserAction::Edit,
            UserAction::Regenerate,
            UserAction::Abort,
        ];

        let ans = Select::new("Choose an action:", options)
            .with_help_message("↑/↓ to navigate, Enter to select, Esc/Ctrl+C to abort")
            .prompt()?;

        Ok(ans)
    }
}
