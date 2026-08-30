use crate::config::Config;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(
        config: &Config,
        diff: &str,
        file_summaries: &[String],
        recent_commits: &[String],
    ) -> (String, String) {
        // 如果用户在配置中显式指定了 custom_prompt
        if let Some(ref custom) = config.custom_prompt {
            let recent_str = recent_commits.join("\n");
            let user_msg = custom
                .replace("{diff}", diff)
                .replace("{recent_commits}", &recent_str)
                .replace("{language}", &config.behavior.language);

            let sys_msg = "You are an expert Git commit message generator. Output ONLY the commit message.".to_string();
            return (sys_msg, user_msg);
        }

        let system_prompt = match config.behavior.template.as_str() {
            "gitmoji" => Self::gitmoji_system_prompt(&config.behavior.language),
            "simple" => Self::simple_system_prompt(&config.behavior.language),
            _ => Self::conventional_system_prompt(&config.behavior.language),
        };

        let mut user_prompt = String::new();

        if !file_summaries.is_empty() {
            user_prompt.push_str("<changed_files_summary>\n");
            for summary in file_summaries {
                user_prompt.push_str("- ");
                user_prompt.push_str(summary);
                user_prompt.push('\n');
            }
            user_prompt.push_str("</changed_files_summary>\n\n");
        }

        if !recent_commits.is_empty() {
            user_prompt.push_str("<recent_commits>\n");
            for commit in recent_commits {
                user_prompt.push_str(commit);
                user_prompt.push('\n');
            }
            user_prompt.push_str("</recent_commits>\n\n");
        }

        user_prompt.push_str("<git_diff>\n");
        user_prompt.push_str(diff);
        user_prompt.push_str("\n</git_diff>");

        (system_prompt, user_prompt)
    }

    fn conventional_system_prompt(language: &str) -> String {
        if language == "en-US" {
            r#"You are an expert Git commit message generator.
Analyze the provided changed files and code diff, then output a clean and concise commit message following Conventional Commits.

Diff Semantics:
1. Lines starting with '-' represent removed content; lines starting with '+' represent added/modified content.
2. If a file only contains '-' lines, it was deleted or cleared. Do not hallucinate added features on deleted files.
3. Consider all modified files to capture the core intent.

Format:
<type>(<optional-scope>): <subject in English, under 72 chars>

- Optional bullet points after an empty line for important details

Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

Strict Rules:
1. Output MUST contain ONLY the commit message. No explanations, no markdown code fences, no prefix text.
2. Keep the subject line concise (under 72 chars), imperative mood, no ending period.
3. Language: en-US.

Examples:
- feat(auth): add OAuth2 login support
- fix(config): resolve endpoint URL normalization error
- chore(config): update default model to qwen3.5-4b and remove obsolete docs
"#
            .to_string()
        } else {
            r#"你是一个专业的 Git 提交信息生成器。请根据输入的变更文件概括与 Git Diff 生成规范的 Commit Message。

Diff 语义准则：
1. '-' 行代表被删除/移除的内容；'+' 行代表新增/修改的内容。
2. 若某文件全为 '-' 行，说明该文件被清空或删除，切勿误判为新增内容。
3. 综合所有修改过的文件，提取最核心的变更意图。

格式要求：
第一行格式必须为 Conventional Commits 规范（英文类型前缀 + 冒号 + 空格 + 中文描述）：
feat(scope): 描述 或 chore(scope): 描述 或 fix: 描述

- 可选正文：若涉及多处细节，空一行后以列表列出具体变更点

可用类型: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

核心规则：
1. 第一行必须严格遵循 `<type>(<scope>): <中文描述>` 格式，类型必须为英文，冒号后有一个空格。
2. 严禁输出任何解释、思考过程、代码块围栏（```）或引导性前缀。
3. 语言使用中文（zh-CN），首行保持精炼（72 字符以内）。

示例：
- chore(config): 更新默认模型为 qwen3.5-4b 并删除旧版中文文档
- feat(auth): 支持 OAuth2 快捷登录与状态校验
- refactor(filter): 优化 Diff 行数截断计算逻辑
"#
            .to_string()
        }
    }

    fn gitmoji_system_prompt(language: &str) -> String {
        if language == "en-US" {
            r#"You are an expert Git commit message generator using Gitmoji format.

Format:
<emoji> <type>(<scope>): <subject>

Common Emojis:
:sparkles: feat
:bug: fix
:recycle: refactor
:books: docs
:package: build/deps
:zap: perf

Rules:
1. Output MUST contain ONLY the single-line commit message.
2. No explanations, no markdown blocks.
3. Example: :sparkles: feat(auth): add OAuth2 authentication
"#
            .to_string()
        } else {
            r#"你是一个 Git 提交信息生成器。根据输入的代码变更（Git Diff）生成 Gitmoji 风格的 Commit Message。

格式要求：
<emoji> <type>(<scope>): <中文描述>

常用 Emoji 映射：
:sparkles: 新功能 (feat)
:bug: 修复缺陷 (fix)
:recycle: 重构代码 (refactor)
:books: 文档变更 (docs)
:package: 依赖更新 (build/deps)
:zap: 性能优化 (perf)

规则：
1. 只输出生成的单行 Commit Message，严禁输出任何多余解释或 Markdown 语法。
2. 示例: :sparkles: feat(auth): 新增 OAuth2 登录支持
"#
            .to_string()
        }
    }

    fn simple_system_prompt(language: &str) -> String {
        if language == "en-US" {
            r#"You are an expert Git commit message generator.
Generate a concise, single-line imperative summary of what changed based on the Git diff.

Rules:
1. Output MUST contain ONLY the commit message.
2. Do NOT use prefixes like 'feat:', no markdown fences, no explanations.
3. Example: Add retry mechanism to client and extend timeout settings
"#
            .to_string()
        } else {
            r#"你是一个 Git 提交信息生成器。根据输入的代码变更（Git Diff）生成简短直观的一句话中文提交信息。

规则：
1. 用祈使语气概括本次变更的核心内容。
2. 严禁使用 'feat:' 等前缀，严禁输出任何 Markdown 块或解释。
3. 示例：优化配置文件加载逻辑并支持环境变量覆盖
"#
            .to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_building_with_commits() {
        let config = Config::default();
        let diff = "diff --git a/src/main.rs b/src/main.rs\n+ let a = 1;";
        let summaries = vec!["src/main.rs: [Modified (+1, -0)]".to_string()];
        let commits = vec!["abc1234 feat: init".to_string()];

        let (sys, user) = PromptBuilder::build(&config, diff, &summaries, &commits);
        assert!(sys.contains("Commit Message"));
        assert!(user.contains("<changed_files_summary>"));
        assert!(user.contains("src/main.rs"));
        assert!(user.contains("<recent_commits>"));
        assert!(user.contains("<git_diff>"));
    }
}
