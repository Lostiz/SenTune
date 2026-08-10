use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent,
};

mod api;
mod cache;
mod commands;
mod db;
mod logging;
mod models;
mod stream;

#[tauri::command]
fn close_to_tray(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

#[tauri::command]
fn quit(app: AppHandle) {
    stream::shutdown();
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            if let Err(error) = logging::init(&data_dir.join("logs")) {
                eprintln!("初始化日志失败：{error}");
            }
            db::init(&data_dir.join("sentune.db"))?;
            stream::server::start(cache::cache_root(&data_dir))?;
            logging::info("应用启动完成");
            // 启动时清理一次，之后每日一次。
            let cache_root_task = cache::cache_root(&data_dir);
            tauri::async_runtime::spawn(async move {
                loop {
                    match cache::cleanup::run_scheduled_cleanup(&cache_root_task) {
                        Ok(deleted) => {
                            logging::info(&format!("缓存清理完成，删除 {deleted} 个文件"));
                        }
                        Err(error) => {
                            logging::error(&format!("缓存清理失败：{error}"));
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
                }
            });

            let toggle_window = MenuItem::with_id(
                app,
                "toggle-window",
                "显示/隐藏",
                true,
                None::<&str>,
            )?;
            let toggle_play = MenuItem::with_id(
                app,
                "toggle-play",
                "播放/暂停",
                true,
                None::<&str>,
            )?;
            let quit_item =
                MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu =
                Menu::with_items(app, &[&toggle_window, &toggle_play, &quit_item])?;

            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().expect("应用图标缺失").clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "toggle-window" => {
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    let _ = window.hide();
                                }
                                _ => {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                    "toggle-play" => {
                        let _ = app.emit("tray-toggle-play", ());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            match window.is_visible() {
                                Ok(true) => {
                                    let _ = window.hide();
                                }
                                _ => {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;
            logging::info("托盘创建完成");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            close_to_tray,
            quit,
            commands::search::search_videos,
            commands::stream::start_stream,
            commands::stream::get_video_detail,
            commands::stream::cancel_stream,
            commands::stream::get_stream_status,
            commands::stream::get_cover_url,
            commands::stream::get_proxy_port,
            commands::library::add_favorite,
            commands::library::remove_favorite,
            commands::library::list_favorites,
            commands::library::create_playlist,
            commands::library::rename_playlist,
            commands::library::delete_playlist,
            commands::library::list_playlists,
            commands::library::get_playlist,
            commands::library::add_to_playlist,
            commands::library::remove_from_playlist,
            commands::library::move_in_playlist,
            commands::library::add_history,
            commands::library::list_history,
            commands::library::clear_history,
            commands::library::list_cached_tracks,
            commands::cache::get_cache_status,
            commands::cache::clear_cache,
            commands::cache::get_cache_settings,
            commands::cache::set_cache_settings,
            commands::cache::pick_cache_dir,
        ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
