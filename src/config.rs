use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::RunArgs;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub provider: ProviderConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
    #[serde(default)]
    pub diff_filter: DiffFilterConfig,
    pub custom_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub temperature: f32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:1234".to_string(),
            api_key: "not-needed".to_string(),
            model: "qwen3.5-2b".to_string(),
            timeout_seconds: 30,
            temperature: 0.2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorConfig {
    pub template: String,
    pub language: String,
    pub auto_stage_if_empty: bool,
    pub max_diff_lines: usize,
    pub max_file_diff_lines: usize,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            template: "conventional".to_string(),
            language: "zh-CN".to_string(),
            auto_stage_if_empty: true,
            max_diff_lines: 500,
            max_file_diff_lines: 150,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffFilterConfig {
    pub ignore_files: Vec<String>,
}

impl Default for DiffFilterConfig {
    fn default() -> Self {
        Self {
            ignore_files: vec![
                "Cargo.lock".to_string(),
                "package-lock.json".to_string(),
                "pnpm-lock.yaml".to_string(),
                "yarn.lock".to_string(),
                "*.min.js".to_string(),
                "*.min.css".to_string(),
                "*.svg".to_string(),
                "*.lock".to_string(),
            ],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            provider: ProviderConfig::default(),
            behavior: BehaviorConfig::default(),
            diff_filter: DiffFilterConfig::default(),
            custom_prompt: None,
        }
    }
}

impl Config {
    pub fn template_toml() -> &'static str {
        r#"# git-msg configuration

[provider]
endpoint = "http://127.0.0.1:1234"
api_key = "not-needed"
model = "qwen3.5-2b"
timeout_seconds = 30
temperature = 0.2

[behavior]
template = "conventional"        # Options: conventional | simple | gitmoji
language = "zh-CN"               # Options: zh-CN | en-US
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
"#
    }

    /// 获取用户全局配置文件的标准路径
    pub fn global_config_path() -> Result<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            Ok(config_dir.join("git-msg").join("config.toml"))
        } else if let Some(home_dir) = dirs::home_dir() {
            Ok(home_dir.join(".config").join("git-msg").join("config.toml"))
        } else {
            anyhow::bail!("Unable to determine user home or config directory");
        }
    }

    /// 确保全局配置文件存在，若不存在则创建默认模板
    pub fn ensure_global_config_exists() -> Result<PathBuf> {
        let path = Self::global_config_path()?;
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
            }
            fs::write(&path, Self::template_toml())
                .with_context(|| format!("Failed to write default config to: {}", path.display()))?;
        }
        Ok(path)
    }

    /// 按照优先级加载并合并配置：
    /// 1. CLI Flags
    /// 2. 环境变量 (GIT_MSG_*)
    /// 3. 项目级配置 (.gitmsg.toml)
    /// 4. 全局配置 (~/.config/git-msg/config.toml)
    /// 5. 默认值
    pub fn load(repo_root: Option<&Path>, cli_args: &RunArgs) -> Result<Self> {
        let mut config = Self::default();

        // 1. 全局配置
        if let Ok(global_path) = Self::global_config_path() {
            if global_path.exists() {
                if let Ok(content) = fs::read_to_string(&global_path) {
                    if let Ok(parsed) = toml::from_str::<Config>(&content) {
                        config = Self::merge(config, parsed);
                    }
                }
            }
        }

        // 2. 项目级配置 (.gitmsg.toml)
        if let Some(root) = repo_root {
            let project_path = root.join(".gitmsg.toml");
            if project_path.exists() {
                let content = fs::read_to_string(&project_path)
                    .with_context(|| format!("Failed to read project config at {}", project_path.display()))?;
                let parsed: Config = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse project config at {}", project_path.display()))?;
                config = Self::merge(config, parsed);
            }
        }

        // 3. 环境变量覆盖
        if let Ok(ep) = env::var("GIT_MSG_ENDPOINT") {
            if !ep.trim().is_empty() {
                config.provider.endpoint = ep;
            }
        }
        if let Ok(m) = env::var("GIT_MSG_MODEL") {
            if !m.trim().is_empty() {
                config.provider.model = m;
            }
        }
        if let Ok(key) = env::var("GIT_MSG_API_KEY").or_else(|_| env::var("OPENAI_API_KEY")) {
            if !key.trim().is_empty() {
                config.provider.api_key = key;
            }
        }

        // 4. CLI Flags 覆盖
        if let Some(ref ep) = cli_args.endpoint {
            config.provider.endpoint = ep.clone();
        }
        if let Some(ref m) = cli_args.model {
            config.provider.model = m.clone();
        }
        if let Some(ref t) = cli_args.template {
            config.behavior.template = t.clone();
        }

        Ok(config)
    }

    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            provider: ProviderConfig {
                endpoint: if overlay.provider.endpoint != "http://127.0.0.1:1234" {
                    overlay.provider.endpoint
                } else {
                    base.provider.endpoint
                },
                api_key: if overlay.provider.api_key != "not-needed" {
                    overlay.provider.api_key
                } else {
                    base.provider.api_key
                },
                model: if overlay.provider.model != "qwen3.5-2b" {
                    overlay.provider.model
                } else {
                    base.provider.model
                },
                timeout_seconds: overlay.provider.timeout_seconds,
                temperature: overlay.provider.temperature,
            },
            behavior: BehaviorConfig {
                template: overlay.behavior.template,
                language: overlay.behavior.language,
                auto_stage_if_empty: overlay.behavior.auto_stage_if_empty,
                max_diff_lines: overlay.behavior.max_diff_lines,
                max_file_diff_lines: overlay.behavior.max_file_diff_lines,
            },
            diff_filter: DiffFilterConfig {
                ignore_files: if !overlay.diff_filter.ignore_files.is_empty() {
                    overlay.diff_filter.ignore_files
                } else {
                    base.diff_filter.ignore_files
                },
            },
            custom_prompt: overlay.custom_prompt.or(base.custom_prompt),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_parsing() {
        let toml_str = Config::template_toml();
        let parsed: Config = toml::from_str(toml_str).expect("Default template should parse cleanly");
        assert_eq!(parsed.provider.endpoint, "http://127.0.0.1:1234");
        assert_eq!(parsed.provider.model, "qwen3.5-2b");
        assert_eq!(parsed.behavior.template, "conventional");
        assert_eq!(parsed.behavior.auto_stage_if_empty, true);
        assert_eq!(parsed.behavior.max_file_diff_lines, 150);
    }
}
