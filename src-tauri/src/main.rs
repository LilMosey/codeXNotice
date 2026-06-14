fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            std::thread::spawn(|| {
                let notifier = codex_notice::notifications::local::MacOsNotifier;
                let loop_config = codex_notice::runtime::default_runtime_loop_config();

                loop {
                    let config = loop_config.to_runtime_config(chrono::Utc::now().timestamp());

                    if let Err(error) = codex_notice::runtime::run_once(&config, &notifier) {
                        eprintln!("CodeX Notice background scan failed: {error}");
                    }

                    std::thread::sleep(loop_config.scan_interval);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run CodeX Notice");
}
