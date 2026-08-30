use regex::Regex;

/// 清洗大模型输出的文本，确保返回规范的 Commit Message
pub fn sanitize_commit_message(raw: &str) -> String {
    let mut text = raw.to_string();

    // 1. 剥离 <think>...</think> 推理链标签（针对 DeepSeek / Qwen 推理模型）
    if let Ok(re_think) = Regex::new(r"(?is)<think>[\s\S]*?</think>") {
        text = re_think.replace_all(&text, "").to_string();
    }

    // 2. 迭代剥离前导提示语与 Markdown 围栏（防止相互嵌套）
    for _ in 0..3 {
        text = strip_preambles(&text);
        text = strip_markdown_fences(&text);
    }

    // 3. 清理首尾空白与多余连续空行
    clean_whitespace(&text)
}

fn strip_markdown_fences(input: &str) -> String {
    let trimmed = input.trim();

    // 如果整个文本以 ``` 开头并以 ``` 结尾
    if trimmed.starts_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 2 && lines.last().is_some_and(|l| l.trim() == "```") {
            let inner_lines = &lines[1..lines.len() - 1];
            return inner_lines.join("\n").trim().to_string();
        }
    }

    trimmed.to_string()
}

fn strip_preambles(input: &str) -> String {
    let preambles = [
        "here is the commit message:",
        "here's the commit message:",
        "commit message:",
        "generated commit message:",
        "git commit message:",
        "生成的提交信息：",
        "生成的提交信息:",
        "提交信息：",
        "提交信息:",
        "本次提交信息：",
        "本次提交信息:",
    ];

    let mut lines: Vec<&str> = input.lines().collect();
    while let Some(first) = lines.first() {
        let first_lower = first.trim().to_lowercase();
        if first_lower.is_empty() {
            lines.remove(0);
            continue;
        }

        let matched = preambles.iter().any(|&p| first_lower.starts_with(p));
        if matched {
            lines.remove(0);
        } else {
            break;
        }
    }

    lines.join("\n").trim().to_string()
}

fn clean_whitespace(input: &str) -> String {
    let mut cleaned_lines = Vec::new();
    let mut consecutive_empty = 0;

    let placeholder_patterns = ["[空一行]", "[空行]", "[optional body]", "[可选正文]"];

    for line in input.lines() {
        let trimmed = line.trim();
        if placeholder_patterns
            .iter()
            .any(|&p| trimmed.eq_ignore_ascii_case(p))
        {
            continue;
        }

        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() {
            consecutive_empty += 1;
            if consecutive_empty <= 1 {
                cleaned_lines.push("");
            }
        } else {
            consecutive_empty = 0;
            cleaned_lines.push(trimmed_end);
        }
    }

    cleaned_lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_message() {
        let input = "feat(auth): add OAuth2 login support";
        assert_eq!(sanitize_commit_message(input), input);
    }

    #[test]
    fn test_think_tag_removal() {
        let input = "<think>\nThinking about changes...\n</think>\nfix(api): resolve timeout issue";
        assert_eq!(
            sanitize_commit_message(input),
            "fix(api): resolve timeout issue"
        );
    }

    #[test]
    fn test_markdown_fence_removal() {
        let input =
            "```git\nfeat(core): initialize project structure\n\n- Add config\n- Add CLI\n```";
        let expected = "feat(core): initialize project structure\n\n- Add config\n- Add CLI";
        assert_eq!(sanitize_commit_message(input), expected);
    }

    #[test]
    fn test_preamble_removal() {
        let input = "Here is the commit message:\n\nfeat(ui): add spinner animation";
        assert_eq!(
            sanitize_commit_message(input),
            "feat(ui): add spinner animation"
        );

        let input_zh = "生成的提交信息：\nfix(db): 修复连接池泄漏";
        assert_eq!(sanitize_commit_message(input_zh), "fix(db): 修复连接池泄漏");
    }

    #[test]
    fn test_combined_messy_output() {
        let input = "<think>Analyzing diff...</think>\n\nHere is the commit message:\n```markdown\nrefactor(config): simplify parser logic\n\n- Remove redundant checks\n```\n";
        let expected = "refactor(config): simplify parser logic\n\n- Remove redundant checks";
        assert_eq!(sanitize_commit_message(input), expected);
    }
}
