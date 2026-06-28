pub mod commands;

use std::sync::Arc;
use tauri::Manager;
use commands::chat::AgentSession;
use commands::persistence::DbState;
use xuflow_core::SessionStore;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // Agent 会话 — 占位凭证，前端必须调用 configure_agent 后发消息。
            let session = AgentSession::new(
                String::from(""),
                String::from("deepseek-v4-pro"),
                String::from("deepseek"),
                handle,
            );
            app.manage(Arc::new(session));

            // SQLite 数据库 — 存储在项目目录下，集中管理。
            let db_path = std::path::PathBuf::from(r"D:\Projects-star\Xuflow\XuFlow-sqlite_content\xuflow.db");
            let store = SessionStore::open(Some(db_path))
                .expect("无法初始化 SQLite 数据库");
            let db_state = Arc::new(DbState {
                store: Arc::new(store),
            });
            app.manage(db_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 聊天相关
            commands::chat::configure_agent,
            commands::chat::get_env_api_keys,
            commands::chat::send_message,
            commands::chat::stop_generation,
            commands::chat::respond_approval,
            commands::chat::generate_title,
            commands::chat::set_context_window,
            commands::chat::set_min_user_turns,
            // Git 审查相关
            commands::git::git_diff_raw,
            commands::git::git_status_raw,
            commands::git::git_add,
            commands::git::git_reset_file,
            commands::git::git_checkout_file,
            commands::git::git_checkout_all,
            commands::git::reveal_in_explorer,
            commands::git::get_working_dir,
            // 数据库路径
            commands::persistence::db_get_path,
            // 数据库连接管理
            commands::persistence::db_connect,
            commands::persistence::db_test_connection,
            commands::persistence::db_disconnect,
            commands::persistence::db_is_connected,
            // 项目 CRUD
            commands::persistence::db_create_project,
            commands::persistence::db_list_projects,
            commands::persistence::db_update_project_name,
            commands::persistence::db_delete_project,
            // 会话 CRUD
            commands::persistence::db_create_session,
            commands::persistence::db_list_sessions,
            commands::persistence::db_list_all_sessions,
            commands::persistence::db_update_session_title,
            commands::persistence::db_delete_session,
            commands::persistence::db_reveal_session,
            // 消息 CRUD
            commands::persistence::db_add_message,
            commands::persistence::db_update_message,
            commands::persistence::db_get_messages,
            commands::persistence::db_clear_messages,
            // 配置读写
            commands::persistence::db_get_config,
            commands::persistence::db_set_config,
            commands::persistence::db_delete_config,
            // 数据迁移
            commands::persistence::db_migrate_from_localstorage,
            commands::persistence::db_is_migrated,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}