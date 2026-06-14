fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            std::thread::spawn(|| {
                let notifier = codex_notice::notifications::local::MacOsNotifier;
                let config = codex_notice::runtime::RuntimeConfig {
                    app_database_path: codex_notice::runtime::default_app_database_path(),
                    codex_directory: codex_notice::runtime::default_codex_directory(),
                    now_epoch_seconds: chrono::Utc::now().timestamp(),
                    delay_ttl_seconds: 86_400,
                };

                if let Err(error) = codex_notice::runtime::run_once(&config, &notifier) {
                    eprintln!("CodeX Notice startup scan failed: {error}");
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run CodeX Notice");
}
