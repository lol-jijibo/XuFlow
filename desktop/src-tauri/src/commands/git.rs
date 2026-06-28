use std::process::Command;

// 审查面板所需的 Git Tauri 命令，直接调用 git CLI 并返回原始输出

/// 返回原始 git diff 输出（供前端 diffParser 解析）
#[tauri::command]
pub async fn git_diff_raw(args: Option<String>) -> Result<String, String> {
    let extra_args: Vec<&str> = args
        .as_deref()
        .map(|s| s.split_whitespace().filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();

    let output = Command::new("git")
        .arg("diff")
        .args(&extra_args)
        .output()
        .map_err(|e| format!("执行 git diff 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.len() > 100_000 {
            // 截断超长 diff，防止前端渲染卡顿
            format!("{}...\n\n[diff 输出已截断，超过 100000 字符]", &stdout[..100_000])
        } else {
            stdout
        })
    } else {
        Err(format!("git diff 返回非零状态: {}", stderr))
    }
}

/// 返回 git status --porcelain 输出
#[tauri::command]
pub async fn git_status_raw() -> Result<String, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("执行 git status 失败: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// 暂存单个文件或目录
#[tauri::command]
pub async fn git_add(files: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["add"])
        .arg(&files)
        .output()
        .map_err(|e| format!("执行 git add 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { format!("已暂存: {}", files) } else { stdout })
    } else {
        Err(format!("git add 失败: {}", stderr))
    }
}

/// 取消暂存单个文件
#[tauri::command]
pub async fn git_reset_file(path: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["reset", "HEAD", "--"])
        .arg(&path)
        .output()
        .map_err(|e| format!("执行 git reset 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { format!("已取消暂存: {}", path) } else { stdout })
    } else {
        Err(format!("git reset 失败: {}", stderr))
    }
}

/// 回退单个文件的变更到 HEAD
#[tauri::command]
pub async fn git_checkout_file(path: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "--"])
        .arg(&path)
        .output()
        .map_err(|e| format!("执行 git checkout 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { format!("已回退: {}", path) } else { stdout })
    } else {
        Err(format!("git checkout 失败: {}", stderr))
    }
}

/// 返回当前工作目录（即 git 仓库根目录）
#[tauri::command]
pub async fn get_working_dir() -> Result<String, String> {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("获取工作目录失败: {}", e))
}

/// 在系统文件管理器中定位并选中指定文件或打开目录
#[tauri::command]
pub async fn reveal_in_explorer(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    #[cfg(target_os = "windows")]
    {
        if p.is_dir() {
            // 目录：直接打开
            Command::new("explorer")
                .arg(&path)
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        } else {
            // 文件：在资源管理器中选中
            Command::new("explorer")
                .args(["/select,", &path])
                .spawn()
                .map_err(|e| format!("打开资源管理器失败: {}", e))?;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if p.is_dir() {
            Command::new("open")
                .arg(&path)
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        } else {
            Command::new("open")
                .args(["-R", &path])
                .spawn()
                .map_err(|e| format!("打开 Finder 失败: {}", e))?;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if p.is_dir() {
            Command::new("xdg-open")
                .arg(&path)
                .spawn()
                .map_err(|e| format!("打开文件管理器失败: {}", e))?;
        } else {
            let parent = std::path::Path::new(&path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.clone());
            Command::new("xdg-open")
                .arg(&parent)
                .spawn()
                .map_err(|e| format!("打开文件管理器失败: {}", e))?;
        }
    }
    Ok(format!("已定位: {}", path))
}

/// 回退所有未暂存的变更
#[tauri::command]
pub async fn git_checkout_all() -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "--", "."])
        .output()
        .map_err(|e| format!("执行 git checkout 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(if stdout.is_empty() { "已回退所有变更".to_string() } else { stdout })
    } else {
        Err(format!("git checkout 失败: {}", stderr))
    }
}
