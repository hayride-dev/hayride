use tauri::{Builder, Manager, Window};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            // TODO :: Remove
            let window = app.get_webview_window("main").expect("Main window not found");
            window.eval(
                r#"
                function waitForPreline() {
                    if (window.HSStaticMethods && typeof window.HSStaticMethods.autoInit === 'function') {
                        window.HSStaticMethods.autoInit();
                        console.log('Preline initialized in Tauri context');
                    } else {
                        console.log('Waiting for Preline to load...');
                        setTimeout(waitForPreline, 100);
                    }
                }
                waitForPreline();
                "#,
            )?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
