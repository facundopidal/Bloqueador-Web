use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use chrono::NaiveTime;

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
pub struct Config {
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub sites: Vec<BlockedSite>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BlockedSite {
    pub url: String,
    pub mode: BlockMode,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockMode {
    Always,
    Scheduled {
        start: NaiveTime,
        end: NaiveTime,
    },
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Config::default());
        }
        let content = fs::read_to_string(path)
            .with_context(|| format!("No se pudo leer el archivo de configuración: {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Error parseando el archivo de configuración TOML: {:?}", path))?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("No se pudo crear el directorio para la configuración: {:?}", parent))?;
        }
        let content = toml::to_string_pretty(self)
            .context("Error serializando la configuración a TOML")?;
        fs::write(path, content)
            .with_context(|| format!("No se pudo escribir el archivo de configuración: {:?}", path))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_empty_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("Failed to serialize empty config");
        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize empty config");
        assert_eq!(config, deserialized);
        assert!(config.sites.is_empty());
    }

    #[test]
    fn test_always_blocked_site_serialization() {
        let config = Config {
            sites: vec![BlockedSite {
                url: "facebook.com".to_string(),
                mode: BlockMode::Always,
            }],
        };
        let toml_str = toml::to_string(&config).expect("Failed to serialize config with always block");
        assert!(toml_str.contains("facebook.com"), "TOML does not contain URL");
        
        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize always config");
        assert_eq!(config, deserialized);
        assert_eq!(deserialized.sites.len(), 1);
        assert_eq!(deserialized.sites[0].url, "facebook.com");
        assert!(matches!(deserialized.sites[0].mode, BlockMode::Always));
    }

    #[test]
    fn test_scheduled_blocked_site_serialization() {
        let start_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end_time = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        let config = Config {
            sites: vec![BlockedSite {
                url: "instagram.com".to_string(),
                mode: BlockMode::Scheduled {
                    start: start_time,
                    end: end_time,
                },
            }],
        };
        let toml_str = toml::to_string(&config).expect("Failed to serialize config with scheduled block");
        assert!(toml_str.contains("instagram.com"), "TOML does not contain URL");
        assert!(toml_str.contains("09:00:00"), "TOML does not contain start time");
        assert!(toml_str.contains("18:00:00"), "TOML does not contain end time");

        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize scheduled config");
        assert_eq!(config, deserialized);
        assert_eq!(deserialized.sites.len(), 1);
        assert_eq!(deserialized.sites[0].url, "instagram.com");
        if let BlockMode::Scheduled { start, end } = deserialized.sites[0].mode {
            assert_eq!(start, start_time);
            assert_eq!(end, end_time);
        } else {
            panic!("Expected BlockMode::Scheduled");
        }
    }

    #[test]
    fn test_load_and_save_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_config.toml");
        
        // Ensure clean state
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        // Test loading non-existent file returns default config
        let loaded_default = Config::load_from_file(&path).unwrap();
        assert_eq!(loaded_default, Config::default());

        // Test saving a config
        let mut config = Config::default();
        config.sites.push(BlockedSite {
            url: "youtube.com".to_string(),
            mode: BlockMode::Always,
        });
        config.save_to_file(&path).expect("Failed to save config to file");

        // Test loading it back
        let loaded = Config::load_from_file(&path).expect("Failed to load config from file");
        assert_eq!(config, loaded);

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_autostart_default_and_serialization() {
        // Test que verifica que autostart se serializa y deserializa correctamente
        let mut config = Config::default();
        assert_eq!(config.autostart, false); // Esto fallará al compilar si no existe el campo

        config.autostart = true;
        let toml_str = toml::to_string(&config).expect("Failed to serialize config with autostart");
        assert!(toml_str.contains("autostart = true") || toml_str.contains("autostart = true\n"));

        let deserialized: Config = toml::from_str(&toml_str).expect("Failed to deserialize config with autostart");
        assert_eq!(deserialized.autostart, true);
    }

    #[test]
    fn test_autostart_missing_in_toml() {
        // Un TOML sin la clave autostart debe deserializarse con autostart = false
        let toml_str = r#"
            [[sites]]
            url = "example.com"
            mode = { type = "always" }
        "#;
        let deserialized: Config = toml::from_str(toml_str).expect("Failed to deserialize config without autostart");
        assert_eq!(deserialized.autostart, false);
    }
}

