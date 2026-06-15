pub mod commands;

use std::sync::Arc;
use tauri::Manager;
use commands::chat::AgentSession;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_sql::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            // Session is created with placeholder credentials — the frontend
            // must call configure_agent before the first send_message.
            let session = AgentSession::new(
                String::from(""),
                String::from("deepseek-v4-pro"),
                String::from("deepseek"),
                handle,
            );
            app.manage(Arc::new(session));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::configure_agent,
            commands::chat::get_env_api_keys,
            commands::chat::send_message,
            commands::chat::stop_generation,
            commands::chat::respond_approval,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
