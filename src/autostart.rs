use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::process::ExitStatus;

/// Modifica el estado de inicio automático en Windows mediante el Programador de Tareas (schtasks).
///
/// Si `enabled` es true, registra una tarea para ejecutarse al iniciar sesión (`onlogon`)
/// con los máximos privilegios (`highest`).
/// Si es false, elimina la tarea programada.
pub fn set_autostart_task(enabled: bool) -> Result<()> {
    set_autostart_task_internal(enabled, std::env::current_exe, |cmd, args| {
        std::process::Command::new(cmd)
            .args(args)
            .status()
            .map_err(|e| anyhow!("Error al ejecutar {}: {}", cmd, e))
    })
}

fn set_autostart_task_internal(
    enabled: bool,
    current_exe_fn: impl FnOnce() -> std::io::Result<PathBuf>,
    run_cmd_fn: impl FnOnce(&str, &[&str]) -> Result<ExitStatus>,
) -> Result<()> {
    if enabled {
        let exe_path = current_exe_fn()?
            .to_str()
            .ok_or_else(|| {
                anyhow!("No se pudo convertir la ruta del ejecutable a formato string válido")
            })?
            .to_string();

        // Para asegurar que schtasks maneje correctamente rutas con espacios,
        // envolvemos la ruta del ejecutable entre comillas dobles escapadas dentro de /tr.
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
            return Err(anyhow!(
                "schtasks falló al crear la tarea. Asegurate de ejecutar la aplicación como Administrador."
            ));
        }
    } else {
        let args = ["/delete", "/tn", "BloqueadorWeb", "/f"];

        let status = run_cmd_fn("schtasks", &args)?;
        if !status.success() {
            return Err(anyhow!(
                "schtasks falló al eliminar la tarea. Asegurate de ejecutar la aplicación como Administrador."
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
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
        let run_mock = |_: &str, _: &[&str]| Ok(mock_exit_status(false));

        let res = set_autostart_task_internal(true, exe_mock, run_mock);
        assert!(res.is_err());
        assert!(
            res.unwrap_err()
                .to_string()
                .contains("schtasks falló al crear")
        );
    }
}
