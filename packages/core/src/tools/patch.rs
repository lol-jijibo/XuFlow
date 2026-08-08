/// apply_patch 工具 —— 基于 unified diff 格式编辑文件。
/// 与 edit 工具互补：edit 做精确字符串替换，apply_patch 做行号定位的 diff 应用，
/// 对空格/缩进偏差更宽容，适合多块编辑和跨文件批量修改。
///
/// 支持的 diff 格式：
///   - 单文件 unified diff（含 @@ -line,count +line,count @@ hunks）
///   - 可包含文件头（--- a/path / +++ b/path），用于校验目标文件
///   - 每个 hunk 包含上下文行（空格前缀）、删除行（- 前缀）、新增行（+ 前缀）
///
/// 模糊匹配策略：
///   hunk header 中的行号作为定位起点，若上下文行在该位置不匹配，
///   则向前后各搜索 5 行寻找匹配位置；找到后以此为基准应用修改。
///   所有 hunks 均成功应用才算成功，任一失败则整体回滚。

use super::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

pub struct ApplyPatchTool;

/// 解析后的单个 diff hunk
#[derive(Debug, Clone)]
struct Hunk {
    /// hunk 头部原文（如 "@@ -10,7 +10,6 @@"）
    header: String,
    /// 原始文件起始行号（1-based）
    old_start: usize,
    /// 原始文件行数（hunk 中上下文+删除行的总行数）
    old_count: usize,
    /// 新文件起始行号（1-based）
    new_start: usize,
    /// 新文件行数（hunk 中上下文+新增行的总行数）
    new_count: usize,
    /// hunk 内部的每一行，保留原始前缀和内容
    lines: Vec<HunkLine>,
}

#[derive(Debug, Clone)]
struct HunkLine {
    /// 行前缀：' ' (上下文), '-' (删除), '+' (新增)
    prefix: char,
    content: String,
}

/// 解析 unified diff 文本，提取所有 hunks 和可选的文件头路径
fn parse_unified_diff(diff_text: &str) -> Result<(Option<String>, Vec<Hunk>), String> {
    let mut target_path: Option<String> = None;
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut current_hunk: Option<Hunk> = None;

    for line in diff_text.lines() {
        // 文件头：提取目标文件路径
        if line.starts_with("+++ ") {
            let path_str = line[4..].trim();
            // 去掉可能的前缀 a/ 或 b/
            let clean_path = path_str
                .strip_prefix("b/")
                .or_else(|| path_str.strip_prefix("a/"))
                .unwrap_or(path_str);
            target_path = Some(clean_path.to_string());
            continue;
        }

        // hunk header：@@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@") {
            // 保存前一个 hunk
            if let Some(h) = current_hunk.take() {
                hunks.push(h);
            }
            // 解析新 hunk header
            match parse_hunk_header(line) {
                Ok(hunk) => {
                    current_hunk = Some(hunk);
                }
                Err(e) => {
                    return Err(format!("无法解析 hunk header '{}': {}", line, e));
                }
            }
            continue;
        }

        // hunk 内部行
        if let Some(ref mut hunk) = current_hunk {
            if line.is_empty() {
                continue; // 跳过空行（hunk 间的分隔）
            }
            let prefix = line.chars().next().unwrap_or(' ');
            let content = if prefix == ' ' || prefix == '-' || prefix == '+' {
                line[1..].to_string()
            } else {
                // 没有标准前缀的行，视为上下文行
                line.to_string()
            };
            let actual_prefix = if prefix == ' ' || prefix == '-' || prefix == '+' {
                prefix
            } else {
                ' '
            };

            hunk.lines.push(HunkLine {
                prefix: actual_prefix,
                content,
            });
        }
    }

    // 保存最后一个 hunk
    if let Some(h) = current_hunk.take() {
        hunks.push(h);
    }

    if hunks.is_empty() {
        return Err("diff 中未找到任何 hunk（期望 @@ -line,count +line,count @@ 头部）".to_string());
    }

    Ok((target_path, hunks))
}

/// 解析 hunk header 行，如 "@@ -10,7 +10,6 @@" 或 "@@ -15,4 +15,5 @@ fn main() {"
fn parse_hunk_header(line: &str) -> Result<Hunk, String> {
    // 找到两个 @@ 之间的内容：处理可能带有上下文提示的 header
    let first_at = line.find("@@").ok_or_else(|| "hunk header 缺少开头的 @@" .to_string())?;
    let after_first = &line[first_at + 2..];
    let second_at = after_first.find("@@").ok_or_else(|| "hunk header 缺少结尾的 @@" .to_string())?;
    let inner = after_first[..second_at].trim();

    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("hunk header 缺少行号范围".to_string());
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old_part)?;
    let (new_start, new_count) = parse_range(new_part)?;

    Ok(Hunk {
        header: line.to_string(),
        old_start,
        old_count,
        new_start,
        new_count,
        lines: Vec::new(),
    })
}

/// 解析 "start" 或 "start,count" 格式
fn parse_range(s: &str) -> Result<(usize, usize), String> {
    if let Some((start_str, count_str)) = s.split_once(',') {
        let start: usize = start_str
            .parse()
            .map_err(|_| format!("无效行号: {}", start_str))?;
        let count: usize = count_str
            .parse()
            .map_err(|_| format!("无效行数: {}", count_str))?;
        Ok((start, count))
    } else {
        let start: usize = s.parse().map_err(|_| format!("无效行号: {}", s))?;
        Ok((start, 1))
    }
}

/// 从原始文件行列表中提取对应 hunk 的上下文行（不带前缀）
fn extract_context_lines(hunk: &Hunk) -> Vec<String> {
    hunk.lines
        .iter()
        .filter(|l| l.prefix == ' ')
        .map(|l| l.content.clone())
        .collect()
}

/// 在文件行列表中搜索与 hunk 上下文匹配的位置。
/// 返回匹配的起始行号（0-based index）或 None。
fn find_hunk_position(file_lines: &[String], hunk: &Hunk) -> Option<usize> {
    let context_lines = extract_context_lines(hunk);
    if context_lines.is_empty() {
        // 无上下文行：使用 hunk header 行号（转换为 0-based）
        if hunk.old_start > 0 {
            return Some((hunk.old_start - 1).min(file_lines.len()));
        }
        return None;
    }

    // 从 old_start 附近开始搜索，逐步扩大范围
    let search_start = if hunk.old_start > 0 {
        (hunk.old_start - 1).min(file_lines.len())
    } else {
        0
    };

    // 搜索范围：前后各 5 行（共 11 个候选位置）
    let search_radius = 5;
    let min_pos = search_start.saturating_sub(search_radius);
    let max_pos = (search_start + search_radius).min(file_lines.len());

    // 优先从精确位置开始搜索
    let mut candidates: Vec<usize> = (min_pos..=max_pos).collect();
    // 排序：离 search_start 越近越优先
    candidates.sort_by_key(|&p| (p as isize - search_start as isize).unsigned_abs());

    for pos in candidates {
        if pos + context_lines.len() > file_lines.len() {
            continue;
        }
        let matches = context_lines
            .iter()
            .enumerate()
            .all(|(i, ctx)| file_lines[pos + i].trim_end() == ctx.trim_end());
        if matches {
            return Some(pos);
        }
    }

    None
}

/// 将一个 hunk 应用到文件行列表上，原地修改
fn apply_hunk(file_lines: &mut Vec<String>, hunk: &Hunk, pos: usize) {
    // 计算当前 hunk 在原始内容中覆盖了多少行
    let old_line_count = hunk
        .lines
        .iter()
        .filter(|l| l.prefix == ' ' || l.prefix == '-')
        .count();

    // 生成新行（来自 hunk 中上下文行 + 新增行）
    let new_lines: Vec<String> = hunk
        .lines
        .iter()
        .filter(|l| l.prefix == ' ' || l.prefix == '+')
        .map(|l| l.content.clone())
        .collect();

    // 替换：移除 old_line_count 行，插入 new_lines
    let end = (pos + old_line_count).min(file_lines.len());
    // 先收集需要保留的行
    let before: Vec<String> = file_lines[..pos].to_vec();
    let after: Vec<String> = file_lines[end..].to_vec();

    *file_lines = before;
    file_lines.extend(new_lines);
    file_lines.extend(after);
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a unified diff patch to a file. Supports standard unified diff format with \
         @@ -line,count +line,count @@ hunks. Tolerates minor line-number drift via \
         context-line fuzzy matching. This is preferred over edit for multi-block changes \
         or when exact string matching is unreliable (e.g., whitespace-sensitive edits)."
    }

    fn is_dangerous(&self) -> bool {
        true
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the file to patch. If the diff contains a +++ header with a path, this parameter is optional (the header path takes precedence)."
                },
                "patch": {
                    "type": "string",
                    "description": "Unified diff content. Must contain at least one @@ -line,count +line,count @@ hunk. Context lines (space-prefixed) are used to locate the correct position in the file."
                }
            },
            "required": ["patch"]
        })
    }

    async fn execute(&self, args: Value) -> ToolResult {
        let patch_text = match args["patch"].as_str() {
            Some(s) => s,
            None => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some("缺少必要参数: patch".into()),
                };
            }
        };

        // 解析 unified diff
        let (diff_path, hunks) = match parse_unified_diff(patch_text) {
            Ok(result) => result,
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("解析 diff 失败: {}", e)),
                };
            }
        };

        // 确定目标文件路径
        let file_path = match args["path"].as_str() {
            Some(p) => p.to_string(),
            None => match diff_path {
                Some(p) => p,
                None => {
                    return ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(
                            "未指定目标文件路径：请在 path 参数中提供，或在 diff 中包含 +++ 文件头".into(),
                        ),
                    };
                }
            },
        };

        // 读取文件
        let original_content = match tokio::fs::read_to_string(&file_path).await {
            Ok(c) => c,
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: String::new(),
                    error: Some(format!("无法读取文件 '{}': {}", file_path, e)),
                };
            }
        };

        // 逐 hunk 定位并应用
        let mut file_lines: Vec<String> =
            original_content.lines().map(|s| s.to_string()).collect();
        let original_lines = file_lines.clone();

        // 从后往前处理 hunks（保持前面的行号不变）
        let mut hunks_with_pos: Vec<(usize, &Hunk)> = Vec::new();
        for hunk in &hunks {
            match find_hunk_position(&file_lines, hunk) {
                Some(pos) => {
                    hunks_with_pos.push((pos, hunk));
                }
                None => {
                    return ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!(
                            "无法定位 hunk '{}'：文件中未找到匹配的上下文行。\
                             请确认文件内容是否与 diff 预期一致。",
                            hunk.header
                        )),
                    };
                }
            }
        }

        // 按位置从后往前排序，避免前面的修改影响后面 hunk 的定位
        hunks_with_pos.sort_by(|a, b| b.0.cmp(&a.0));

        let applied_count = hunks_with_pos.len();
        for (pos, hunk) in &hunks_with_pos {
            apply_hunk(&mut file_lines, hunk, *pos);
        }

        // 确保结尾换行符与原始文件一致
        let ends_with_newline = original_content.ends_with('\n');
        let mut new_content = file_lines.join("\n");
        if ends_with_newline {
            new_content.push('\n');
        }

        // 写入文件
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            if !parent.as_os_str().is_empty() {
                if let Err(e) = tokio::fs::create_dir_all(parent).await {
                    return ToolResult {
                        success: false,
                        content: String::new(),
                        error: Some(format!("无法创建父目录: {}", e)),
                    };
                }
            }
        }

        match tokio::fs::write(&file_path, &new_content).await {
            Ok(_) => ToolResult {
                success: true,
                content: format!(
                    "成功应用 {} 个 hunk 到 '{}'。",
                    applied_count, file_path
                ),
                error: None,
            },
            Err(e) => ToolResult {
                success: false,
                content: String::new(),
                error: Some(format!("无法写入文件 '{}': {}", file_path, e)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header_basic() {
        let hunk = parse_hunk_header("@@ -10,7 +10,6 @@").unwrap();
        assert_eq!(hunk.old_start, 10);
        assert_eq!(hunk.old_count, 7);
        assert_eq!(hunk.new_start, 10);
        assert_eq!(hunk.new_count, 6);
    }

    #[test]
    fn test_parse_hunk_header_no_count() {
        let hunk = parse_hunk_header("@@ -1 +1,3 @@").unwrap();
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 3);
    }

    #[test]
    fn test_parse_hunk_header_context() {
        let hunk = parse_hunk_header("@@ -15,4 +15,5 @@ fn main() {").unwrap();
        assert_eq!(hunk.old_start, 15);
        assert_eq!(hunk.old_count, 4);
        assert_eq!(hunk.new_start, 15);
        assert_eq!(hunk.new_count, 5);
    }

    #[test]
    fn test_parse_unified_diff_single_hunk() {
        let diff = "--- a/foo.txt\n+++ b/foo.txt\n@@ -1,3 +1,3 @@\n line1\n-line2\n+line2_new\n line3\n";
        let (path, hunks) = parse_unified_diff(diff).unwrap();
        assert_eq!(path.unwrap(), "foo.txt");
        assert_eq!(hunks.len(), 1);
        // 4 行：2 上下文 + 1 删除 + 1 新增
        assert_eq!(hunks[0].lines.len(), 4);
    }

    #[test]
    fn test_parse_unified_diff_multi_hunk() {
        let diff = "@@ -1,3 +1,3 @@\n a\n-b\n+c\n d\n@@ -10,2 +10,3 @@\n x\n-y\n+z\n w\n";
        let (_path, hunks) = parse_unified_diff(diff).unwrap();
        assert_eq!(hunks.len(), 2);
    }

    #[test]
    fn test_parse_unified_diff_no_hunks() {
        let diff = "just some text\nwithout hunks\n";
        let result = parse_unified_diff(diff);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_hunk_position_exact() {
        let file_lines: Vec<String> = vec![
            "line1".into(), "line2".into(), "line3".into(), "line4".into(),
        ];
        let diff = "@@ -2,2 +2,2 @@\n line2\n line3\n";
        let (_, hunks) = parse_unified_diff(diff).unwrap();
        let pos = find_hunk_position(&file_lines, &hunks[0]);
        assert_eq!(pos, Some(1)); // 0-based
    }

    #[test]
    fn test_find_hunk_position_offset() {
        let file_lines: Vec<String> = vec![
            "line0".into(), "line1".into(), "line2".into(), "line3".into(),
            "line4".into(),
        ];
        // hunk 说从行 5 开始，但实际上下文在行 1-3
        let diff = "@@ -5,3 +5,3 @@\n line1\n line2\n line3\n";
        let (_, hunks) = parse_unified_diff(diff).unwrap();
        let pos = find_hunk_position(&file_lines, &hunks[0]);
        assert_eq!(pos, Some(1)); // 模糊匹配到实际位置
    }

    #[test]
    fn test_apply_hunk_simple() {
        let mut file_lines: Vec<String> = vec![
            "line1".into(), "old_line".into(), "line3".into(),
        ];
        let diff = "@@ -2,1 +2,1 @@\n-old_line\n+new_line\n";
        let (_, hunks) = parse_unified_diff(diff).unwrap();
        apply_hunk(&mut file_lines, &hunks[0], 1);
        assert_eq!(file_lines, vec!["line1", "new_line", "line3"]);
    }

    #[test]
    fn test_apply_hunk_add_lines() {
        let mut file_lines: Vec<String> = vec![
            "line1".into(), "line2".into(),
        ];
        let diff = "@@ -1,1 +1,3 @@\n line1\n+line1.5\n+line1.8\n";
        let (_, hunks) = parse_unified_diff(diff).unwrap();
        apply_hunk(&mut file_lines, &hunks[0], 0);
        assert_eq!(file_lines, vec!["line1", "line1.5", "line1.8", "line2"]);
    }
}