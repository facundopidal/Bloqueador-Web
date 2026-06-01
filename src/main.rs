#![windows_subsystem = "windows"]

use anyhow::Result;
use chrono::NaiveTime;
#[cfg(target_os = "windows")]
use is_elevated::is_elevated;

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn is_elevated() -> bool {
    unsafe { libc::getuid() == 0 }
}

use log::{LevelFilter, error, info};
use rfd::{MessageDialog, MessageLevel};
use simplelog::{Config as LogConfig, WriteLogger};
use std::sync::mpsc;

mod hosts;
use hosts::{normalize_url, read_hosts_data, save_hosts_data};

mod config;
use config::{BlockMode, BlockedSite, Config};

mod autostart;

// Importar módulo compilado de Slint
slint::include_modules!();

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.iter().any(|arg| arg == "--daemon");

    if is_daemon {
        run_daemon()
    } else {
        run_ui()
    }
}

#[cfg(target_os = "windows")]
fn get_config_path() -> std::path::PathBuf {
    match std::env::current_exe() {
        Ok(mut path) => {
            path.pop();
            path.push("config.toml");
            path
        }
        Err(_) => std::path::PathBuf::from("config.toml"),
    }
}

#[cfg(not(target_os = "windows"))]
fn get_config_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/etc/bloqueador-web/config.toml")
}

fn run_daemon() -> Result<()> {
    #[cfg(target_os = "windows")]
    let log_path = match std::env::current_exe() {
        Ok(path) => path.with_extension("log"),
        Err(_) => std::path::PathBuf::from("bloqueador_fallback.log"),
    };

    #[cfg(not(target_os = "windows"))]
    let log_path = std::path::PathBuf::from("/var/log/bloqueador-web.log");

    let log_file_result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .or_else(|_| {
            let fallback_path = std::path::PathBuf::from("bloqueador_daemon_fallback.log");
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&fallback_path)
        });

    if let Ok(file) = log_file_result {
        let _ = WriteLogger::init(LevelFilter::Info, LogConfig::default(), file);
    }

    info!("--------------------------------------------------");
    info!("Iniciando Daemon del Bloqueador Web...");
    info!("Log inicializado en: {:?}", log_path);

    let config_path = get_config_path();

    if !config_path.exists() {
        info!("config.toml no existe. Inicializando desde archivo hosts actual...");
        let mut initial_config = Config::default();
        if let Ok(hosts_data) = read_hosts_data() {
            for url in hosts_data.blocked_urls {
                initial_config.sites.push(BlockedSite {
                    url,
                    mode: BlockMode::Always,
                });
            }
        }
        if let Err(e) = initial_config.save_to_file(&config_path) {
            error!("No se pudo guardar la configuración inicial del Daemon: {}", e);
        }
    }

    info!("Bucle principal del Daemon iniciado (polling de 10 segundos).");
    loop {
        match Config::load_from_file(&config_path) {
            Ok(config) => {
                let now = chrono::Local::now().time();
                let mut active_blocked = std::collections::HashSet::new();

                for site in config.sites {
                    let is_blocked = match &site.mode {
                        BlockMode::Always => true,
                        BlockMode::Scheduled { start, end } => {
                            if start <= end {
                                now >= *start && now <= *end
                            } else {
                                now >= *start || now <= *end
                            }
                        }
                    };

                    if is_blocked {
                        let normalized = normalize_url(&site.url);
                        if let Some(stripped) = normalized.strip_prefix("www.") {
                            active_blocked.insert(stripped.to_string());
                            active_blocked.insert(normalized.clone());
                        } else {
                            active_blocked.insert(format!("www.{}", normalized));
                            active_blocked.insert(normalized);
                        }
                    }
                }

                match read_hosts_data() {
                    Ok(mut hosts_data) => {
                        if hosts_data.blocked_urls != active_blocked {
                            info!("Cambió el estado de bloqueos. Actualizando hosts...");
                            hosts_data.blocked_urls = active_blocked;
                            if let Err(e) = save_hosts_data(&hosts_data) {
                                error!("Error del Daemon al escribir hosts: {}", e);
                            } else {
                                info!("Archivo hosts actualizado con éxito por el Daemon.");
                            }
                        }
                    }
                    Err(e) => {
                        error!("Daemon no pudo leer el archivo hosts: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Daemon no pudo cargar la configuración: {}", e);
            }
        }

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

fn run_ui() -> Result<()> {
    #[cfg(target_os = "windows")]
    let log_path = match std::env::current_exe() {
        Ok(path) => path.with_extension("log"),
        Err(_) => std::path::PathBuf::from("bloqueador_fallback.log"),
    };

    #[cfg(not(target_os = "windows"))]
    let log_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let mut path = std::path::PathBuf::from(home);
        path.push(".config");
        path.push("bloqueador-web");
        let _ = std::fs::create_dir_all(&path);
        path.push("bloqueador-web-ui.log");
        path
    };

    let log_file_result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path);

    match log_file_result {
        Ok(file) => {
            let _ = WriteLogger::init(LevelFilter::Info, LogConfig::default(), file);
        }
        Err(e) => {
            MessageDialog::new()
                .set_title("Error de Log")
                .set_description(format!(
                    "No se pudo crear el archivo de log en {:?}\nError: {}",
                    log_path, e
                ))
                .set_level(MessageLevel::Error)
                .show();
        }
    }

    info!("--------------------------------------------------");
    info!("Iniciando UI de Bloqueador Web...");
    info!("Log inicializado en: {:?}", log_path);

    // 1. Verificación de Administrador (Solo en Windows al arrancar)
    #[cfg(target_os = "windows")]
    {
        if !is_elevated() {
            info!("La aplicación no tiene permisos de administrador. Mostrando diálogo.");
            MessageDialog::new()
                .set_title("Error de Permisos")
                .set_description("Este programa necesita ejecutarse como Administrador para modificar el archivo hosts.")
                .set_level(MessageLevel::Error)
                .show();
            return Ok(());
        }
        info!("Permisos de administrador confirmados.");
    }

    let (tx, rx) = mpsc::channel::<()>();

    #[cfg(target_os = "windows")]
    start_scheduler(rx);

    #[cfg(not(target_os = "windows"))]
    {
        // En Linux, simplemente descartamos los mensajes enviados al scheduler local
        std::thread::spawn(move || {
            while rx.recv().is_ok() {}
        });
    }

    let config_path = get_config_path();

    // 2. Levantar la ventana de Slint
    let app = AppWindow::new()?;

    // Sincronizar UI al arrancar
    sync_ui_from_config(&app, &config_path);

    // 3. Crear Tray Icon (Solo en Windows)
    #[cfg(target_os = "windows")]
    let _tray_icon = {
        info!("Configurando Tray Icon...");
        let icon = load_icon();
        match build_tray_menu() {
            Ok(menu) => {
                match tray_icon::TrayIconBuilder::new()
                    .with_menu(Box::new(menu))
                    .with_menu_on_left_click(false)
                    .with_tooltip("Bloqueador Web")
                    .with_icon(icon)
                    .build()
                {
                    Ok(tray) => Some(tray),
                    Err(e) => {
                        error!("No se pudo inicializar el Tray Icon: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("No se pudo construir el menú del Tray Icon: {}", e);
                None
            }
        }
    };

    #[cfg(target_os = "windows")]
    let has_tray = _tray_icon.is_some();
    #[cfg(not(target_os = "windows"))]
    let has_tray = false;

    // 4. Interceptar el cierre (X) para ocultar la ventana en vez de cerrar el proceso (solo si hay Tray Icon)
    let app_weak = app.as_weak();
    app.window().on_close_requested(move || {
        if has_tray {
            if let Some(app) = app_weak.upgrade() {
                info!("Ventana cerrada por el usuario. Ocultando ventana ya que hay Tray Icon...");
                app.hide().unwrap();
            }
            slint::CloseRequestResponse::KeepWindowShown
        } else {
            info!("Ventana cerrada por el usuario. Cerrando la aplicación.");
            let _ = slint::quit_event_loop();
            slint::CloseRequestResponse::HideWindow
        }
    });

    // 5. Configurar callbacks del Tray Icon (solo en Windows)
    #[cfg(target_os = "windows")]
    {
        let menu_channel = tray_icon::menu::MenuEvent::receiver();
        let tray_channel = tray_icon::TrayIconEvent::receiver();
        let app_weak_tray = app.as_weak();
        std::thread::spawn(move || {
            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    let id = event.id.0.as_str();
                    let app_weak = app_weak_tray.clone();
                    if id == "open_app" {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(app) = app_weak.upgrade() {
                                app.show().unwrap();
                            }
                        });
                    } else if id == "exit_app" {
                        let _ = slint::invoke_from_event_loop(move || {
                            slint::quit_event_loop().unwrap();
                        });
                        break;
                    }
                }

                if let Ok(event) = tray_channel.try_recv() {
                    match event {
                        tray_icon::TrayIconEvent::DoubleClick { .. } => {
                            let app_weak = app_weak_tray.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak.upgrade() {
                                    app.show().unwrap();
                                }
                            });
                        }
                        tray_icon::TrayIconEvent::Click {
                            button: tray_icon::MouseButton::Left,
                            button_state: tray_icon::MouseButtonState::Up,
                            ..
                        } => {
                            let app_weak = app_weak_tray.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(app) = app_weak.upgrade() {
                                    app.show().unwrap();
                                }
                            });
                        }
                        _ => {}
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }

    // 6. Configurar Callbacks de la UI
    let app_weak = app.as_weak();
    let config_path_clone = config_path.clone();
    let tx_clone = tx.clone();
    app.on_add_site(move |url, mode_type, start_time, end_time| {
        let app = match app_weak.upgrade() {
            Some(app) => app,
            None => return,
        };

        let url_trimmed = url.trim();
        if url_trimmed.is_empty() {
            app.set_error_message(slint::SharedString::from("La URL no puede estar vacía."));
            return;
        }

        if !url_trimmed.contains('.') || url_trimmed.contains(' ') {
            app.set_error_message(slint::SharedString::from(
                "Por favor ingresá un dominio válido (ej: facebook.com).",
            ));
            return;
        }

        let normalized = normalize_url(url_trimmed);

        let mode = match mode_type.as_str() {
            "scheduled" => {
                let start_parsed = NaiveTime::parse_from_str(start_time.trim(), "%H:%M")
                    .or_else(|_| NaiveTime::parse_from_str(start_time.trim(), "%H:%M:%S"));
                let end_parsed = NaiveTime::parse_from_str(end_time.trim(), "%H:%M")
                    .or_else(|_| NaiveTime::parse_from_str(end_time.trim(), "%H:%M:%S"));

                match (start_parsed, end_parsed) {
                    (Ok(start), Ok(end)) => BlockMode::Scheduled { start, end },
                    _ => {
                        app.set_error_message(slint::SharedString::from(
                            "Formato de hora inválido. Usar HH:MM.",
                        ));
                        return;
                    }
                }
            }
            _ => BlockMode::Always,
        };

        let mut config = Config::load_from_file(&config_path_clone).unwrap_or_default();

        if config
            .sites
            .iter()
            .any(|s| normalize_url(&s.url) == normalized)
        {
            app.set_error_message(slint::SharedString::from("Este sitio ya está en la lista."));
            return;
        }

        config.sites.push(BlockedSite {
            url: normalized,
            mode,
        });

        if let Err(e) = config.save_to_file(&config_path_clone) {
            app.set_error_message(slint::SharedString::from(format!(
                "Error al guardar: {}",
                e
            )));
            return;
        }

        app.set_error_message(slint::SharedString::from(""));
        sync_ui_from_config(&app, &config_path_clone);
        let _ = tx_clone.send(());
    });

    let app_weak = app.as_weak();
    let config_path_clone = config_path.clone();
    let tx_clone = tx.clone();
    app.on_remove_site(move |url| {
        let app = match app_weak.upgrade() {
            Some(app) => app,
            None => return,
        };

        let normalized = normalize_url(url.trim());
        let mut config = Config::load_from_file(&config_path_clone).unwrap_or_default();

        let initial_len = config.sites.len();
        config.sites.retain(|s| normalize_url(&s.url) != normalized);

        if config.sites.len() < initial_len {
            if let Err(e) = config.save_to_file(&config_path_clone) {
                app.set_error_message(slint::SharedString::from(format!(
                    "Error al guardar: {}",
                    e
                )));
                return;
            }
            sync_ui_from_config(&app, &config_path_clone);
            let _ = tx_clone.send(());
        }
    });

    let app_weak = app.as_weak();
    let config_path_clone = config_path.clone();
    app.on_toggle_autostart(move |enabled| {
        let app = match app_weak.upgrade() {
            Some(app) => app,
            None => return,
        };

        if let Err(e) = autostart::set_autostart_task(enabled) {
            app.set_error_message(slint::SharedString::from(format!(
                "Error de Autostart: {}",
                e
            )));
            let current_config = Config::load_from_file(&config_path_clone).unwrap_or_default();
            app.set_autostart_enabled(current_config.autostart);
            return;
        }

        let mut config = Config::load_from_file(&config_path_clone).unwrap_or_default();
        config.autostart = enabled;

        if let Err(e) = config.save_to_file(&config_path_clone) {
            app.set_error_message(slint::SharedString::from(format!(
                "Error al guardar configuración: {}",
                e
            )));
            let current_config = Config::load_from_file(&config_path_clone).unwrap_or_default();
            app.set_autostart_enabled(current_config.autostart);
            return;
        }

        app.set_error_message(slint::SharedString::from(""));
        app.set_autostart_enabled(enabled);
    });

    // 7. Correr event loop
    info!("Mostrando ventana principal y corriendo event loop...");
    app.show()?;
    slint::run_event_loop_until_quit()?;

    info!("Event loop finalizado. Saliendo de la aplicación.");
    Ok(())
}

fn sync_ui_from_config(app: &AppWindow, config_path: &std::path::Path) {
    let config = Config::load_from_file(config_path).unwrap_or_default();
    app.set_autostart_enabled(config.autostart);

    let sites_model = slint::VecModel::default();
    for site in &config.sites {
        let (mode_type, start_time, end_time) = match &site.mode {
            BlockMode::Always => (
                slint::SharedString::from("always"),
                slint::SharedString::from(""),
                slint::SharedString::from(""),
            ),
            BlockMode::Scheduled { start, end } => (
                slint::SharedString::from("scheduled"),
                slint::SharedString::from(start.format("%H:%M").to_string()),
                slint::SharedString::from(end.format("%H:%M").to_string()),
            ),
        };
        sites_model.push(BlockedSiteItem {
            url: slint::SharedString::from(&site.url),
            mode_type,
            start_time,
            end_time,
        });
    }
    app.set_sites(slint::ModelRc::new(sites_model));
}

#[cfg(target_os = "windows")]
fn build_tray_menu() -> Result<tray_icon::menu::Menu> {
    use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};

    let menu = Menu::new();
    let item_open = MenuItem::with_id(MenuId::new("open_app"), "Abrir", true, None);
    let item_exit = MenuItem::with_id(MenuId::new("exit_app"), "Salir", true, None);

    menu.append(&item_open)?;
    menu.append(&PredefinedMenuItem::separator())?;
    menu.append(&item_exit)?;

    Ok(menu)
}

#[cfg(target_os = "windows")]
fn load_icon() -> tray_icon::Icon {
    let icon_file_bytes = include_bytes!("../assets/icon.png");

    let image = image::load_from_memory(icon_file_bytes)
        .expect("Falló al cargar el icono")
        .into_rgba8();

    let (width, height) = image.dimensions();
    let rgba = image.into_raw();

    tray_icon::Icon::from_rgba(rgba, width, height).expect("Falló al crear icono")
}

#[cfg(target_os = "windows")]
fn start_scheduler(rx: mpsc::Receiver<()>) {
    std::thread::spawn(move || {
        info!("Iniciando hilo del scheduler en Windows...");
        let config_path = get_config_path();

        if !config_path.exists() {
            info!("config.toml no existe. Inicializando desde archivo hosts actual...");
            let mut initial_config = Config::default();
            if let Ok(hosts_data) = read_hosts_data() {
                for url in hosts_data.blocked_urls {
                    initial_config.sites.push(BlockedSite {
                        url,
                        mode: BlockMode::Always,
                    });
                }
            }
            if let Err(e) = initial_config.save_to_file(&config_path) {
                error!("No se pudo guardar la configuración inicial: {}", e);
            }
        }

        loop {
            match Config::load_from_file(&config_path) {
                Ok(config) => {
                    let now = chrono::Local::now().time();
                    let mut active_blocked = std::collections::HashSet::new();

                    for site in config.sites {
                        let is_blocked = match &site.mode {
                            BlockMode::Always => true,
                            BlockMode::Scheduled { start, end } => {
                                if start <= end {
                                    now >= *start && now <= *end
                                } else {
                                    now >= *start || now <= *end
                                }
                            }
                        };

                        if is_blocked {
                            let normalized = normalize_url(&site.url);
                            if let Some(stripped) = normalized.strip_prefix("www.") {
                                active_blocked.insert(stripped.to_string());
                                active_blocked.insert(normalized.clone());
                            } else {
                                active_blocked.insert(format!("www.{}", normalized));
                                active_blocked.insert(normalized);
                            }
                        }
                    }

                    match read_hosts_data() {
                        Ok(mut hosts_data) => {
                            if hosts_data.blocked_urls != active_blocked {
                                info!("La lista de bloqueados activos cambió. Actualizando hosts...");
                                hosts_data.blocked_urls = active_blocked;
                                if let Err(e) = save_hosts_data(&hosts_data) {
                                    error!("Error actualizando archivo hosts desde el scheduler: {}", e);
                                } else {
                                    info!("Archivo hosts actualizado exitosamente por el scheduler.");
                                }
                            }
                        }
                        Err(e) => {
                            error!("Scheduler no pudo leer el archivo hosts: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Scheduler no pudo cargar la configuración: {}", e);
                }
            }

            let _ = rx.recv_timeout(std::time::Duration::from_secs(30));
        }
    });
}
