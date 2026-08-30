use glob::Pattern;

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub file_path: String,
    pub raw_lines: Vec<String>,
    pub is_binary: bool,
}

/// 将 Git 原始 Diff 按照文件拆分、过滤忽略文件并执行两级截断保护
pub fn filter_and_truncate_diff(
    raw_diff: &str,
    ignore_patterns: &[String],
    max_file_diff_lines: usize,
    max_diff_lines: usize,
) -> String {
    let file_diffs = parse_diff_files(raw_diff);

    // 构建 glob patterns
    let patterns: Vec<Pattern> = ignore_patterns
        .iter()
        .filter_map(|p| Pattern::new(p).ok())
        .collect();

    let mut result_chunks = Vec::new();
    let mut total_lines = 0;
    let mut globally_truncated = false;

    for diff in file_diffs {
        // 1. 检查是否匹配 ignore glob
        if is_file_ignored(&diff.file_path, &patterns) {
            continue;
        }

        // 2. 如果已达到全局总行数限制
        if total_lines >= max_diff_lines {
            globally_truncated = true;
            break;
        }

        // 3. 处理二进制文件
        if diff.is_binary {
            let summary = format!("diff --git a/{} b/{}\n[Binary file modified]", diff.file_path, diff.file_path);
            total_lines += 2;
            result_chunks.push(summary);
            continue;
        }

        // 4. 单文件行数截断
        let file_quota = max_file_diff_lines.min(max_diff_lines.saturating_sub(total_lines));

        let mut lines_to_take = diff.raw_lines.clone();
        let was_file_truncated = lines_to_take.len() > file_quota;

        if was_file_truncated {
            let truncated_count = lines_to_take.len() - file_quota;
            lines_to_take.truncate(file_quota);
            lines_to_take.push(format!("[... {} lines truncated in {} ...]", truncated_count, diff.file_path));
        }

        total_lines += lines_to_take.len();
        result_chunks.push(lines_to_take.join("\n"));

        if total_lines >= max_diff_lines {
            globally_truncated = true;
            break;
        }
    }

    if globally_truncated {
        result_chunks.push(format!("\n[... Remaining diff truncated (exceeded global limit of {} lines) ...]", max_diff_lines));
    }

    result_chunks.join("\n\n")
}

fn is_file_ignored(file_path: &str, patterns: &[Pattern]) -> bool {
    let normalized = file_path.replace('\\', "/");
    let file_name = normalized.split('/').last().unwrap_or(&normalized);

    patterns.iter().any(|p| {
        p.matches(&normalized) || p.matches(file_name)
    })
}

fn parse_diff_files(raw_diff: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;

    for line in raw_diff.lines() {
        if line.starts_with("diff --git ") {
            if let Some(prev) = current_file.take() {
                files.push(prev);
            }

            let path = extract_file_path(line);
            current_file = Some(FileDiff {
                file_path: path,
                raw_lines: vec![line.to_string()],
                is_binary: false,
            });
        } else if let Some(ref mut cur) = current_file {
            if line.contains("Binary files ") && line.ends_with("differ") {
                cur.is_binary = true;
            }
            cur.raw_lines.push(line.to_string());
        }
    }

    if let Some(prev) = current_file {
        files.push(prev);
    }

    files
}

fn extract_file_path(diff_header: &str) -> String {
    // 例如: "diff --git a/src/main.rs b/src/main.rs"
    if let Some(pos) = diff_header.rfind(" b/") {
        return diff_header[pos + 3..].trim().to_string();
    }
    diff_header.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_pattern_filtering() {
        let diff = r#"diff --git a/Cargo.lock b/Cargo.lock
index 111..222 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,3 +1,3 @@
-foo = "1.0.0"
+foo = "1.0.1"
diff --git a/src/main.rs b/src/main.rs
index 333..444 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,2 +1,3 @@
+println!("hello");
"#;

        let ignored = vec!["Cargo.lock".to_string(), "*.lock".to_string()];
        let filtered = filter_and_truncate_diff(diff, &ignored, 100, 500);

        assert!(!filtered.contains("Cargo.lock"));
        assert!(filtered.contains("src/main.rs"));
        assert!(filtered.contains("println!(\"hello\");"));
    }

    #[test]
    fn test_per_file_truncation() {
        let mut lines = vec!["diff --git a/src/big.rs b/src/big.rs".to_string(), "--- a/src/big.rs".to_string(), "+++ b/src/big.rs".to_string()];
        for i in 0..200 {
            lines.push(format!("+ let x{} = {};", i, i));
        }
        let diff = lines.join("\n");

        let filtered = filter_and_truncate_diff(&diff, &[], 50, 500);
        assert!(filtered.contains("[... 153 lines truncated in src/big.rs ...]"));
    }
}
