use std::fs;
use std::path::Path;
use tokio::process::Command;
use crate::config::Config;

pub fn get_java_args(config: &Config) -> Vec<String> {
    let mut args = Vec::new();

    // Base Profile (Memory etc)
    args.push(format!("-Xms{}", config.memory));
    args.push(format!("-Xmx{}", config.memory));

    // Aikar's Flags
    args.extend(
        [
            "-XX:+UseG1GC",
            "-XX:+ParallelRefProcEnabled",
            "-XX:MaxGCPauseMillis=200",
            "-XX:+UnlockExperimentalVMOptions",
            "-XX:+DisableExplicitGC",
            "-XX:+AlwaysPreTouch",
            "-XX:G1NewSizePercent=30",
            "-XX:G1MaxNewSizePercent=40",
            "-XX:G1HeapRegionSize=8M",
            "-XX:G1ReservePercent=20",
            "-XX:G1HeapWastePercent=5",
            "-XX:G1MixedGCCountTarget=4",
            "-XX:InitiatingHeapOccupancyPercent=15",
            "-XX:G1MixedGCLiveThresholdPercent=90",
            "-XX:G1RSetUpdatingPauseTimePercent=5",
            "-XX:SurvivorRatio=32",
            "-XX:+PerfDisableSharedMem",
            "-XX:MaxTenuringThreshold=1",
            "-Dusing.aikars.flags=https://mcflags.emc.gs",
            "-Daikars.new.flags=true",
        ]
        .map(String::from),
    );

    if config.flavor.contains("forge") {
        // Forge uses @unix_args.txt for the real launch
        let entries = fs::read_dir("libraries/net/minecraftforge/forge").ok();
        if let Some(mut entries) = entries {
            if let Some(Ok(entry)) = entries.next() {
                let args_file = entry.path().join("unix_args.txt");
                if args_file.exists() {
                    args.push(format!("@{}", args_file.display()));
                }
            }
        }
    } else {
        args.push("-jar".to_string());
        if config.flavor.contains("fabric") {
            args.push(
                config
                    .loader_jar
                    .clone()
                    .unwrap_or_else(|| "fabric-loader.jar".to_string()),
            );
        } else {
            args.push(config.vanilla_jar.clone());
        }
    }
    args.push("nogui".to_string());

    args
}

pub async fn handle_forge_install(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let libs_path = Path::new("libraries");
    if !libs_path.exists() {
        println!("[MC-MANAGER] Forge libraries not found. Running installer...");
        let installer_path = config.loader_jar.as_ref().ok_or("LOADER_JAR not set for Forge")?;
        let status = Command::new(&config.java_bin)
            .arg("-jar")
            .arg(installer_path)
            .arg("--installServer")
            .status()
            .await?;

        if !status.success() {
            return Err("Forge installer failed".into());
        }
        println!("[MC-MANAGER] Forge installation complete.");
    }
    Ok(())
}

pub async fn handle_eula(config: &Config) -> Result<(), std::io::Error> {
    let eula_path = Path::new("eula.txt");

    if config.eula {
        println!("[MC-MANAGER] EULA accepted via environment variable.");
        fs::write(eula_path, "eula=true\n")
            .map_err(|e| std::io::Error::new(e.kind(), format!("Failed to write eula.txt: {}", e)))?;
    } else if !eula_path.exists() {
        println!("[MC-MANAGER] ERROR: EULA not accepted. Set EULA=TRUE environment variable.");
    }
    Ok(())
}
