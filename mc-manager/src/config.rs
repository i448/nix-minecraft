use std::env;

pub struct Config {
    pub flavor: String,
    pub vanilla_jar: String,
    pub loader_jar: Option<String>,
    pub java_bin: String,
    #[cfg(feature = "mods")]
    pub mods_dir: Option<String>,
    pub overrides_dir: Option<String>,
    pub memory: String,
    pub eula: bool,
}

impl Config {
    pub fn from_env() -> Self {
        let config = Self {
            flavor: env::var("FLAVOR").unwrap_or_else(|_| "vanilla".to_string()),
            vanilla_jar: env::var("VANILLA_JAR").expect("VANILLA_JAR must be set"),
            loader_jar: env::var("LOADER_JAR").ok().filter(|s| !s.is_empty()),
            java_bin: env::var("JAVA_BIN")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "java".to_string()),
            #[cfg(feature = "mods")]
            mods_dir: env::var("MODS_DIR").ok().filter(|s| !s.is_empty()),
            overrides_dir: env::var("OVERRIDES_DIR").ok().filter(|s| !s.is_empty()),
            memory: env::var("MEMORY").unwrap_or_else(|_| "4G".to_string()),
            eula: env::var("EULA").unwrap_or_default().to_uppercase() == "TRUE",
        };

        println!("[MC-MANAGER] Flavor: {}", config.flavor);
        println!("[MC-MANAGER] Vanilla JAR: {}", config.vanilla_jar);
        if let Some(l) = &config.loader_jar {
            println!("[MC-MANAGER] Loader JAR: {}", l);
        }
        config
    }
}
