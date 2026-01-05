mod config;
mod launcher;
mod sync;
mod util;

use crate::config::Config;
use crate::launcher::{get_java_args, handle_eula, handle_forge_install};
use crate::sync::{setup_links, sync_overrides};
#[cfg(feature = "mods")]
use crate::sync::sync_mods;

use std::process::Stdio;
use std::path::Path;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("[MC-MANAGER] Starting Minecraft Server Manager...");

    // 1. Load Configuration
    let config = Config::from_env();

    // 1.1 Verify JARs exist
    if !Path::new(&config.vanilla_jar).exists() {
        println!("[MC-MANAGER] WARNING: Vanilla JAR target {} does not exist in the container!", config.vanilla_jar);
    }
    if let Some(lj) = &config.loader_jar {
        if !Path::new(lj).exists() {
            println!("[MC-MANAGER] WARNING: Loader JAR target {} does not exist in the container!", lj);
        }
    }

    // 2. Setup Symlinks and Installers (Hybrid-Atomic)
    setup_links(&config.flavor, &config.vanilla_jar, config.loader_jar.as_deref())?;

    // 3. Sync Mods and Overrides if it's a Modpack
    #[cfg(feature = "mods")]
    if let Some(md) = &config.mods_dir {
        sync_mods(md)?;
    }
    if let Some(od) = &config.overrides_dir {
        sync_overrides(od)?;
    }

    // 4. Handle Forge Installation if needed
    if config.flavor == "forge" || config.flavor == "modpack-forge" {
        handle_forge_install(&config).await?;
    }

    // 5. Handle EULA
    handle_eula(&config).await?;

    // 6. Prepare Java Arguments
    let java_args = get_java_args(&config);

    // 7. Launch Minecraft
    println!("[MC-MANAGER] Launching {} with args: {:?}", config.java_bin, java_args);
    let mut child = Command::new(&config.java_bin)
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
