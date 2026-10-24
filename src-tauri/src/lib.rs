mod commands;
mod launch;
mod prism;
mod state;
mod theme;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::detect_prism,
            commands::set_prism_install,
            commands::refresh_instances,
            commands::list_instances,
            commands::load_demo_data,
            commands::get_instance_icon,
            commands::launch_instance,
            commands::stop_run,
            commands::list_runs,
            commands::get_run_logs,
            commands::list_themes,
            commands::get_theme_css,
        ])
        .setup(|app| {
            theme::watch_themes(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run PrismDeck");
}
