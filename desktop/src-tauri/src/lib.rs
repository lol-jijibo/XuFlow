pub mod commands;

use std::sync::Arc;
use tauri::Manager;
use commands::chat::SessionManager;
use commands::persistence::DbState;
use xuflow_core::SessionStore;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            // 会话管理器 — 维护多会话 AgentLoop 池，支持并行对话。
            let session_manager = SessionManager::new(handle);
            app.manage(Arc::new(session_manager));

            // SQLite 数据库 — 使用 Tauri 跨平台应用数据目录，自动创建父目录。
            let db_path = app.path().app_data_dir()
                .expect("无法解析应用数据目录")
                .join("xuflow.db");
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
            // 会话生命周期
            commands::chat::restore_session,
            commands::chat::close_session,
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
            // 回收站操作
            commands::persistence::db_restore_session,
            commands::persistence::db_permanent_delete_session,
            commands::persistence::db_list_trash_sessions,
            commands::persistence::db_purge_expired_trash,
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