use std::env;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[MC-MANAGER] Starting Minecraft Server Manager...");

    // 1. Handle Environments
    let flavor = env::var("FLAVOR").unwrap_or_else(|_| "vanilla".to_string());
    let vanilla_jar = env::var("VANILLA_JAR").expect("VANILLA_JAR must be set");
    let loader_jar = env::var("LOADER_JAR").ok();
    let java_bin = env::var("JAVA_BIN").unwrap_or_else(|_| "java".to_string());
    #[cfg(feature = "mods")]
    let mods_dir = env::var("MODS_DIR").ok();
    let overrides_dir = env::var("OVERRIDES_DIR").ok();

    // 2. Setup Symlinks and Installers (Hybrid-Atomic)
    setup_links(&flavor, &vanilla_jar, loader_jar.as_deref())?;

    // 3. Sync Mods and Overrides if it's a Modpack
    #[cfg(feature = "mods")]
    if let Some(md) = mods_dir {
        if !md.is_empty() {
            sync_mods(&md)?;
        }
    }
    if let Some(od) = overrides_dir {
        if !od.is_empty() {
            sync_overrides(&od)?;
        }
    }

    // 4. Handle Forge Installation if needed
    if flavor == "forge" || flavor == "modpack-forge" {
        handle_forge_install(loader_jar.as_deref().unwrap()).await?;
    }

    // 5. Handle EULA
    handle_eula().await?;

    // 6. Prepare Java Arguments
    let java_args = get_java_args(&flavor);

    // 7. Launch Minecraft
    println!("[MC-MANAGER] Launching {} with args: {:?}", java_bin, java_args);
    let mut child = Command::new(java_bin)
        .args(&java_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn Java process");

    let mut stdin = child.stdin.take().unwrap();
    let mut reader = BufReader::new(tokio::io::stdin()).lines();

    // 8. Signal Handling (SIGTERM)
    let mut sigterm = signal(SignalKind::terminate())?;
    let mut stdin_open = true;

    println!("[MC-MANAGER] Server process started. Waiting for signals...");

    loop {
        tokio::select! {
             _ = sigterm.recv() => {
                println!("[MC-MANAGER] Received SIGTERM. Stopping server gracefully...");
                stdin.write_all(b"stop\n").await?;
                stdin.flush().await?;
                break;
            }
            res = reader.next_line(), if stdin_open => {
                match res {
                    Ok(Some(line)) => {
                        stdin.write_all(line.as_bytes()).await?;
                        stdin.write_all(b"\n").await?;
                        stdin.flush().await?;
                    }
                    Ok(None) => {
                        println!("[MC-MANAGER] Stdin closed.");
                        stdin_open = false;
                    }
                    Err(e) => eprintln!("[MC-MANAGER] Error reading stdin: {}", e),
                }
            }
            status = child.wait() => {
                println!("[MC-MANAGER] Server process exited with status: {:?}", status);
                return Ok(());
            }
        }
    }

    // Ensure we wait for the process to actually exit
    let _ = child.wait().await;
    println!("[MC-MANAGER] Shutdown complete.");

    Ok(())
}

#[cfg(feature = "mods")]
fn sync_mods(source: &str) -> std::io::Result<()> {
    let source_path = Path::new(source);
    if !source_path.exists() {
        println!("[MC-MANAGER] WARNING: Mods directory {} not found. Skipping.", source);
        return Ok(());
    }

    println!("[MC-MANAGER] Syncing mods from Nix store...");
    let target_dir = Path::new("mods");
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).ok();
    } else {
        // Clean existing symlinks in mods/
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_symlink() {
                fs::remove_file(path)?;
            }
        }
    }

    // Create new symlinks
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = target_dir.join(entry.file_name());
        symlink(entry.path(), target)?;
    }
    Ok(())
}

fn sync_overrides(source: &str) -> Result<(), std::io::Error> {
    let source_path = Path::new(source);
    if !source_path.exists() {
        println!("[MC-MANAGER] WARNING: Overrides directory {} not found. Skipping.", source);
        return Ok(());
    }
    println!("[MC-MANAGER] Syncing overrides from Nix store...");
    copy_recursive(source_path, Path::new("."))
}

fn copy_recursive(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    if source.is_dir() {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let dest = target.join(path.file_name().ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Invalid filename"))?);
            copy_recursive(&path, &dest)?;
        }
    } else {
        fs::copy(source, target)?;
    }
    Ok(())
}

async fn handle_forge_install(installer_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let java_bin = env::var("JAVA_BIN").unwrap_or_else(|_| "java".to_string());
    let libs_path = Path::new("libraries");
    if !libs_path.exists() {
        println!("[MC-MANAGER] Forge libraries not found. Running installer...");
        let status = Command::new(java_bin)
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

fn setup_links(flavor: &str, vanilla: &str, loader: Option<&str>) -> Result<(), std::io::Error> {
    let server_jar_path = Path::new("server.jar");
    if server_jar_path.exists() || server_jar_path.is_symlink() {
        fs::remove_file(server_jar_path)?;
    }
    symlink(vanilla, server_jar_path)?;

    if flavor == "fabric" || flavor == "forge" || flavor.starts_with("modpack") {
        if let Some(l) = loader {
            let jar_name = if flavor.contains("fabric") { "fabric-loader.jar" } else { "forge-installer.jar" };
            let loader_jar_path = Path::new(jar_name);
            if loader_jar_path.exists() || loader_jar_path.is_symlink() {
                fs::remove_file(loader_jar_path)?;
            }
            symlink(l, loader_jar_path)?;
        }
    }
    Ok(())
}

async fn handle_eula() -> Result<(), std::io::Error> {
    let eula_accepted = env::var("EULA").unwrap_or_default().to_uppercase() == "TRUE";
    let eula_path = Path::new("eula.txt");

    if eula_accepted {
        println!("[MC-MANAGER] EULA accepted via environment variable.");
        fs::write(eula_path, "eula=true\n")?;
    } else if !eula_path.exists() {
        println!("[MC-MANAGER] ERROR: EULA not accepted. Set EULA=TRUE environment variable.");
    }
    Ok(())
}

fn get_java_args(flavor: &str) -> Vec<String> {
    let mut args = Vec::new();

    // Base Profile (Memory etc)
    let memory = env::var("MEMORY").unwrap_or_else(|_| "4G".to_string());
    args.push(format!("-Xms{}", memory));
    args.push(format!("-Xmx{}", memory));

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
    .map(String::from)
);

    if flavor.contains("forge") {
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
        if flavor.contains("fabric") {
            args.push("fabric-loader.jar".to_string());
        } else {
            args.push("server.jar".to_string());
        }
    }
    args.push("nogui".to_string());

    args
}
