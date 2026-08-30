use crate::config::Config;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build(config: &Config, diff: &str, recent_commits: &[String]) -> (String, String) {
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
Analyze the provided code diff and output a clean commit message following Conventional Commits.

Format:
<type>(<scope>): <subject>

- Optional body bullet points after an empty line

Allowed types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

Strict Rules:
1. Output MUST contain ONLY the commit message. No explanations, no markdown code fences, no prefix text.
2. Keep the subject line concise (under 72 chars), imperative mood, no ending period.
3. Language: en-US.

Examples:
- feat(auth): add OAuth2 login support
- fix(config): resolve endpoint URL normalization error
- refactor(api): simplify request pipeline and error handling
"#
            .to_string()
        } else {
            r#"你是一个 Git 提交信息生成器。根据输入的代码变更（Git Diff）生成规范的 Commit Message。

格式要求：
<type>(<scope>): <简要总结（首行不超过72字符）>

- 如果需要补充细节，在空一行后以列表形式列出重要变更点

可用类型: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert

核心规则：
1. 第一行必须严格遵循 `<type>(<scope>): <中文描述>` 格式，类型必须为英文，冒号后有一个空格。
2. 严禁输出任何解释、思考过程、代码块围栏（```）或引导性前缀。
3. 语言使用中文（zh-CN），首行保持精炼（72 字符以内）。

示例：
- feat(auth): 支持 OAuth2 快捷登录与状态校验
- fix(config): 修复端点 URL 解析与斜杠补全逻辑
- refactor(api): 简化网络请求管道并优化错误提示
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
        let commits = vec!["abc1234 feat: init".to_string()];

        let (sys, user) = PromptBuilder::build(&config, diff, &commits);
        assert!(sys.contains("Commit Message"));
        assert!(user.contains("<recent_commits>"));
        assert!(user.contains("abc1234 feat: init"));
        assert!(user.contains("<git_diff>"));
    }
}
