use anyhow::{anyhow, Result};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::process::ExitStatus;

/// Modifica el estado de inicio automático en Windows mediante el Programador de Tareas (schtasks)
/// o en Linux mediante la creación de un archivo `.desktop` en el directorio autostart.
#[cfg(target_os = "windows")]
pub fn set_autostart_task(enabled: bool) -> Result<()> {
    set_autostart_task_internal(
        enabled,
        std::env::current_exe,
        |cmd, args| {
            std::process::Command::new(cmd)
                .args(args)
                .status()
                .map_err(|e| anyhow!("Error al ejecutar {}: {}", cmd, e))
        }
    )
}

#[cfg(target_os = "windows")]
fn set_autostart_task_internal(
    enabled: bool,
    current_exe_fn: impl FnOnce() -> std::io::Result<PathBuf>,
    run_cmd_fn: impl FnOnce(&str, &[&str]) -> Result<ExitStatus>,
) -> Result<()> {
    if enabled {
        let exe_path = current_exe_fn()?
            .to_str()
            .ok_or_else(|| anyhow!("No se pudo convertir la ruta del ejecutable a formato string válido"))?
            .to_string();
        
        let tr_value = format!("\"{}\"", exe_path);
        let args = [
            "/create",
            "/tn",
            "BloqueadorWeb",
            "/tr",
            &tr_value,
            "/sc",
            "onlogon",
            "/rl",
            "highest",
            "/f",
        ];

        let status = run_cmd_fn("schtasks", &args)?;
        if !status.success() {
            return Err(anyhow!("schtasks falló al crear la tarea. Asegurate de ejecutar la aplicación como Administrador."));
        }
    } else {
        let args = [
            "/delete",
            "/tn",
            "BloqueadorWeb",
            "/f",
        ];

        let status = run_cmd_fn("schtasks", &args)?;
        if !status.success() {
            return Err(anyhow!("schtasks falló al eliminar la tarea. Asegurate de ejecutar la aplicación como Administrador."));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_autostart_task(enabled: bool) -> Result<()> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("No se pudo obtener la variable de entorno HOME"))?;
    
    let mut autostart_dir = PathBuf::from(home);
    autostart_dir.push(".config");
    autostart_dir.push("autostart");
    
    let mut desktop_file = autostart_dir.clone();
    desktop_file.push("bloqueador-web.desktop");
    
    if enabled {
        std::fs::create_dir_all(&autostart_dir)
            .map_err(|e| anyhow!("No se pudo crear el directorio de autostart: {}", e))?;
            
        let exe_path = std::env::current_exe()?
            .to_str()
            .ok_or_else(|| anyhow!("No se pudo convertir la ruta del ejecutable a string"))?
            .to_string();
            
        let desktop_content = format!(
            "[Desktop Entry]\n\
            Type=Application\n\
            Name=Bloqueador Web\n\
            Exec=\"{}\"\n\
            Hidden=false\n\
            NoDisplay=false\n\
            X-GNOME-Autostart-enabled=true\n\
            Comment=Bloqueador de sitios web para concentrarse\n",
            exe_path
        );
        
        std::fs::write(&desktop_file, desktop_content)
            .map_err(|e| anyhow!("No se pudo escribir el archivo .desktop: {}", e))?;
    } else if desktop_file.exists() {
        std::fs::remove_file(&desktop_file)
            .map_err(|e| anyhow!("No se pudo eliminar el archivo .desktop: {}", e))?;
    }
    
    Ok(())
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    fn mock_exit_status(success: bool) -> ExitStatus {
        let code = if success { "0" } else { "1" };
        std::process::Command::new("cmd")
            .args(["/c", &format!("exit {}", code)])
            .status()
            .unwrap()
    }

    #[test]
    fn test_set_autostart_enable_success() {
        let exe_mock = || Ok(PathBuf::from(r"C:\Program Files\WebBlocker\blocker.exe"));
        let run_mock = |cmd: &str, args: &[&str]| {
            assert_eq!(cmd, "schtasks");
            assert_eq!(args[0], "/create");
            assert_eq!(args[1], "/tn");
            assert_eq!(args[2], "BloqueadorWeb");
            assert_eq!(args[3], "/tr");
            assert_eq!(args[4], "\"C:\\Program Files\\WebBlocker\\blocker.exe\"");
            assert_eq!(args[5], "/sc");
            assert_eq!(args[6], "onlogon");
            assert_eq!(args[7], "/rl");
            assert_eq!(args[8], "highest");
            assert_eq!(args[9], "/f");
            Ok(mock_exit_status(true))
        };

        let res = set_autostart_task_internal(true, exe_mock, run_mock);
        assert!(res.is_ok());
    }

    #[test]
    fn test_set_autostart_disable_success() {
        let exe_mock = || Ok(PathBuf::from(""));
        let run_mock = |cmd: &str, args: &[&str]| {
            assert_eq!(cmd, "schtasks");
            assert_eq!(args[0], "/delete");
            assert_eq!(args[1], "/tn");
            assert_eq!(args[2], "BloqueadorWeb");
            assert_eq!(args[3], "/f");
            Ok(mock_exit_status(true))
        };

        let res = set_autostart_task_internal(false, exe_mock, run_mock);
        assert!(res.is_ok());
    }

    #[test]
    fn test_set_autostart_enable_failure() {
        let exe_mock = || Ok(PathBuf::from("blocker.exe"));
        let run_mock = |_: &str, _: &[&str]| {
            Ok(mock_exit_status(false))
        };

        let res = set_autostart_task_internal(true, exe_mock, run_mock);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("schtasks falló al crear"));
    }
}
