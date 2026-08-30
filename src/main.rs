mod cli;
mod config;
mod filter;
mod git;
mod llm;
mod prompt;
mod sanitizer;
mod ui;

use std::fs;
use std::process;
use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;

use crate::cli::{Cli, Commands};
use crate::config::Config;
use crate::filter::filter_and_truncate_diff;
use crate::git::GitManager;
use crate::llm::LlmClient;
use crate::prompt::PromptBuilder;
use crate::ui::{Ui, UserAction};

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{} {}", "Error:".red().bold(), err);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // 1. 处理子命令
    if let Some(command) = cli.command {
        match command {
            Commands::Config(args) => {
                let config_path = Config::ensure_global_config_exists()?;
                if args.show_path {
                    println!("{}", config_path.display());
                    return Ok(());
                }

                println!("Opening configuration in editor: {}", config_path.display().to_string().cyan());
                let initial_content = fs::read_to_string(&config_path).unwrap_or_default();
                let edited = GitManager::open_editor_for_message(&initial_content)?;
                fs::write(&config_path, edited).context("Failed to write updated configuration")?;
                println!("{}", "Configuration saved successfully.".green());
                return Ok(());
            }
            Commands::Init => {
                let repo_root = GitManager::get_repo_root()?;
                let project_config_path = repo_root.join(".gitmsg.toml");
                if project_config_path.exists() {
                    println!(
                        "{} Project config already exists at: {}",
                        "Warning:".yellow().bold(),
                        project_config_path.display()
                    );
                    return Ok(());
                }

                fs::write(&project_config_path, Config::template_toml())
                    .with_context(|| format!("Failed to create {}", project_config_path.display()))?;
                println!(
                    "{} Created project config at {}",
                    "Success:".green().bold(),
                    project_config_path.display()
                );
                return Ok(());
            }
        }
    }

    // 2. 主流程：生成 Commit Message
    let repo_root = match GitManager::get_repo_root() {
        Ok(root) => Some(root),
        Err(err) => {
            eprintln!("{} {}", "Error:".red().bold(), err);
            process::exit(1);
        }
    };

    let config = Config::load(repo_root.as_deref(), &cli.run_args)?;

    // 收集 Diff
    let (raw_diff, need_stage) = GitManager::collect_diff(config.behavior.auto_stage_if_empty)?;
    if raw_diff.trim().is_empty() {
        println!("{}", "No changes detected to commit.".yellow());
        return Ok(());
    }

    // 过滤与截断 Diff
    let filtered_diff = filter_and_truncate_diff(
        &raw_diff,
        &config.diff_filter.ignore_files,
        config.behavior.max_file_diff_lines,
        config.behavior.max_diff_lines,
    );

    if filtered_diff.trim().is_empty() {
        println!("{}", "No relevant changes found after applying diff filter.".yellow());
        return Ok(());
    }

    // 提取最近提交记录（风格参考）
    let recent_commits = GitManager::get_recent_commits(3);

    // 初始化大模型客户端
    let llm_client = LlmClient::new(config.provider.clone())?;

    // 如果指定了 --dry-run，仅打印消息
    if cli.run_args.dry_run {
        let (sys, user) = PromptBuilder::build(&config, &filtered_diff, &recent_commits);
        let msg = llm_client.generate_commit_message(&sys, &user)?;
        println!("{}", msg);
        return Ok(());
    }

    // 交互式生成循环（支持重新生成）
    loop {
        let spinner = Ui::create_spinner("Analyzing diff & generating commit message...");
        let (sys, user) = PromptBuilder::build(&config, &filtered_diff, &recent_commits);
        let generated_msg = match llm_client.generate_commit_message(&sys, &user) {
            Ok(msg) => {
                spinner.finish_and_clear();
                msg
            }
            Err(e) => {
                spinner.finish_and_clear();
                return Err(e);
            }
        };

        Ui::render_commit_message_box(&generated_msg);

        // 如果开启了 --yes，直接提交
        if cli.run_args.yes {
            if need_stage {
                GitManager::stage_all()?;
            }
            GitManager::commit_with_file(&generated_msg)?;
            println!("{}", "Commit successful!".green().bold());
            return Ok(());
        }

        // 交互选择菜单
        match Ui::prompt_action()? {
            UserAction::Commit => {
                if need_stage {
                    GitManager::stage_all()?;
                }
                GitManager::commit_with_file(&generated_msg)?;
                println!("{}", "Commit successful!".green().bold());
                return Ok(());
            }
            UserAction::Edit => {
                let edited_msg = GitManager::open_editor_for_message(&generated_msg)?;
                if edited_msg.trim().is_empty() {
                    println!("{}", "Commit aborted (empty commit message).".yellow());
                    return Ok(());
                }
                if need_stage {
                    GitManager::stage_all()?;
                }
                GitManager::commit_with_file(&edited_msg)?;
                println!("{}", "Commit successful with edited message!".green().bold());
                return Ok(());
            }
            UserAction::Regenerate => {
                println!("{}", "Regenerating commit message...\n".cyan());
                continue;
            }
            UserAction::Abort => {
                println!("{}", "Commit aborted by user.".yellow());
                return Ok(());
            }
        }
    }
}
