use anyhow::{Context, Result};
use log::{info, warn};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

const BEGIN_MARKER: &str = "# BEGIN BLOQUEADOR-WEB";
const END_MARKER: &str = "# END BLOQUEADOR-WEB";
const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";

pub struct HostsData {
    pub base_lines: Vec<String>,
    pub blocked_urls: HashSet<String>,
}

pub fn read_hosts_data() -> Result<HostsData> {
    read_hosts_data_from_path(HOSTS_PATH)
}

pub fn read_hosts_data_from_path<P: AsRef<Path>>(path: P) -> Result<HostsData> {
    let path = path.as_ref();
    info!("Leyendo archivo hosts desde: {:?}", path);
    let file = File::open(path).context("No se pudo abrir el archivo hosts")?;
    let reader = BufReader::new(file);

    let mut base_lines = Vec::new();
    let mut blocked_urls = HashSet::new();
    let mut inside_block = false;

    for line_result in reader.lines() {
        let line = line_result.context("Error leyendo una línea del archivo hosts")?;
        let trimmed = line.trim();

        if trimmed == BEGIN_MARKER {
            inside_block = true;
            continue;
        }
        if trimmed == END_MARKER {
            inside_block = false;
            continue;
        }

        if inside_block {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && (parts[0] == "127.0.0.1" || parts[0] == "0.0.0.0") {
                blocked_urls.insert(parts[1].to_string());
            }
        } else {
            base_lines.push(line);
        }
    }

    info!(
        "Lectura de hosts completada. {} URLs bloqueadas encontradas.",
        blocked_urls.len()
    );
    Ok(HostsData {
        base_lines,
        blocked_urls,
    })
}

pub fn save_hosts_data(data: &HostsData) -> Result<()> {
    save_hosts_data_to_path(HOSTS_PATH, data)
}

pub fn save_hosts_data_to_path<P: AsRef<Path>>(path: P, data: &HostsData) -> Result<()> {
    let path = path.as_ref();
    info!(
        "Guardando {} URLs bloqueadas en el archivo hosts en {:?}",
        data.blocked_urls.len(),
        path
    );

    // 1. Crear backup antes de modificar si existe
    if path.exists() {
        let backup_path = path.with_extension("bak");
        if let Err(e) = fs::copy(path, &backup_path) {
            warn!("No se pudo crear backup de hosts: {}", e);
        } else {
            info!("Backup de hosts creado en: {:?}", backup_path);
        }
    }

    let mut new_content = data.base_lines.clone();

    if !data.blocked_urls.is_empty() {
        new_content.push(BEGIN_MARKER.to_string());
        let mut sorted_urls: Vec<_> = data.blocked_urls.iter().collect();
        sorted_urls.sort(); // Ordenar para que el contenido sea determinista
        for url in sorted_urls {
            new_content.push(format!("127.0.0.1 {}", url));
        }
        new_content.push(END_MARKER.to_string());
    }

    // 2. Escritura atómica usando archivo temporal
    let temp_path = path.with_extension("tmp");
    info!("Escribiendo en archivo temporal: {:?}", temp_path);
    {
        let mut file = File::create(&temp_path).context("No se pudo crear archivo temporal")?;
        for line in new_content {
            writeln!(file, "{}", line)?;
        }
    }

    fs::rename(&temp_path, path)
        .context("Error al reemplazar el archivo hosts original. Comprueba tu antivirus.")?;
    info!("Archivo hosts actualizado exitosamente.");
    Ok(())
}

pub fn normalize_url(url: &str) -> String {
    url.trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("http://facebook.com/"), "facebook.com");
        assert_eq!(normalize_url("https://www.youtube.com"), "www.youtube.com");
        assert_eq!(normalize_url("  Google.com  "), "google.com");
    }

    #[test]
    fn test_save_hosts_data_dynamic_path() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_hosts");
        if path.exists() {
            let _ = fs::remove_file(&path);
        }

        // Create a dummy base hosts content
        fs::write(&path, "127.0.0.1 localhost\n::1 localhost\n").unwrap();

        let mut blocked_urls = HashSet::new();
        blocked_urls.insert("twitter.com".to_string());
        blocked_urls.insert("www.twitter.com".to_string());

        let data = HostsData {
            base_lines: vec![
                "127.0.0.1 localhost".to_string(),
                "::1 localhost".to_string(),
            ],
            blocked_urls,
        };

        // This will fail to compile because `save_hosts_data_to_path` doesn't exist yet.
        save_hosts_data_to_path(&path, &data).unwrap();

        // Verify the file content
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# BEGIN BLOQUEADOR-WEB"));
        assert!(content.contains("127.0.0.1 twitter.com"));
        assert!(content.contains("127.0.0.1 www.twitter.com"));
        assert!(content.contains("# END BLOQUEADOR-WEB"));

        let _ = fs::remove_file(&path);
    }
}
