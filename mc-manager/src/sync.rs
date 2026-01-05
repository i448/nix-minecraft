use std::fs;
use std::path::Path;
use crate::util::copy_recursive;

pub fn setup_links(flavor: &str, vanilla: &str, loader: Option<&str>) -> Result<(), std::io::Error> {
    let server_jar_path = Path::new("server.jar");

    let resolved_vanilla = if Path::new(vanilla).is_symlink() {
        fs::canonicalize(vanilla).unwrap_or_else(|_| Path::new(vanilla).to_path_buf())
    } else {
        Path::new(vanilla).to_path_buf()
    };

    let source_meta = fs::metadata(&resolved_vanilla).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("Failed to get metadata for vanilla jar {}: {}", resolved_vanilla.display(), e),
        )
    })?;

    let target_meta = fs::metadata(server_jar_path).ok();
    let needs_copy = match target_meta {
        Some(tm) => tm.len() != source_meta.len(),
        None => true,
    };

    if needs_copy {
        if server_jar_path.exists() || server_jar_path.is_symlink() {
            fs::remove_file(server_jar_path).ok();
        }
        fs::copy(&resolved_vanilla, server_jar_path).map_err(|e| {
            std::io::Error::new(
                e.kind(),
                format!("Failed to copy vanilla jar to server.jar: {}", e),
            )
        })?;
    } else {
        println!("[MC-MANAGER] Skipping server.jar (identical size)");
    }

    if flavor == "fabric" || flavor == "forge" || flavor.starts_with("modpack") {
        if let Some(l) = loader {
            let jar_name = if flavor.contains("fabric") {
                "fabric-loader.jar"
            } else {
                "forge-installer.jar"
            };
            let loader_jar_path = Path::new(jar_name);

            let resolved_loader = if Path::new(l).is_symlink() {
                fs::canonicalize(l).unwrap_or_else(|_| Path::new(l).to_path_buf())
            } else {
                Path::new(l).to_path_buf()
            };

            let source_meta = fs::metadata(&resolved_loader).map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("Failed to get metadata for loader jar {}: {}", resolved_loader.display(), e),
                )
            })?;

            let target_meta = fs::metadata(loader_jar_path).ok();
            let needs_loader_copy = match target_meta {
                Some(tm) => tm.len() != source_meta.len(),
                None => true,
            };

            if needs_loader_copy {
                if loader_jar_path.exists() || loader_jar_path.is_symlink() {
                    fs::remove_file(loader_jar_path).ok();
                }
                fs::copy(&resolved_loader, loader_jar_path).map_err(|e| {
                    std::io::Error::new(
                        e.kind(),
                        format!("Failed to copy loader jar to {}: {}", jar_name, e),
                    )
                })?;
            } else {
                println!("[MC-MANAGER] Skipping {} (identical size)", jar_name);
            }
        }
    }
    Ok(())
}

#[cfg(feature = "mods")]
pub fn sync_mods(source: &str) -> std::io::Result<()> {
    let source_path = Path::new(source);
    if !source_path.exists() {
        println!("[MC-MANAGER] WARNING: Mods directory {} not found. Skipping.", source);
        return Ok(());
    }

    println!("[MC-MANAGER] Syncing mods from Nix store...");
    let target_dir = Path::new("mods");
    if !target_dir.exists() {
        let _ = fs::create_dir_all(target_dir);
    }

    // Collect active mod names to identify stale ones
    let active_mods: std::collections::HashSet<std::ffi::OsString> = fs::read_dir(source)?
        .filter_map(|e| e.ok().map(|e| e.file_name()))
        .collect();

    // Clean stale mods
    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        if !active_mods.contains(&entry.file_name()) {
            println!("[MC-MANAGER] Removing stale mod: {}", entry.file_name().to_string_lossy());
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }
    }

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = target_dir.join(entry.file_name());
        let mut source_file = entry.path();

        if source_file.is_symlink() {
            match fs::canonicalize(&source_file) {
                Ok(resolved) => {
                    source_file = resolved;
                }
                Err(e) => {
                    println!(
                        "[MC-MANAGER] WARNING: Failed to resolve symlink {}: {}. Using original path.",
                        source_file.display(),
                        e
                    );
                }
            }
        }

        match fs::metadata(&source_file) {
            Ok(source_meta) => {
                let target_meta = fs::metadata(&target).ok();
                if let Some(tm) = target_meta {
                    if tm.len() == source_meta.len() {
                        println!(
                            "[MC-MANAGER] Skipping mod (identical size): {}",
                            entry.file_name().to_string_lossy()
                        );
                        continue;
                    }
                }

                println!(
                    "[MC-MANAGER] Copying mod: {} (Size: {} bytes)",
                    entry.file_name().to_string_lossy(),
                    source_meta.len()
                );
                
                if let Err(e) = fs::copy(&source_file, &target) {
                    println!(
                        "[MC-MANAGER] ERROR: Failed to copy mod {} to {}: {}",
                        source_file.display(),
                        target.display(),
                        e
                    );
                    return Err(e);
                }
            }
            Err(e) => {
                println!(
                    "[MC-MANAGER] ERROR: Cannot access mod source {}: {}",
                    source_file.display(),
                    e
                );
            }
        }
    }
    Ok(())
}

pub fn sync_overrides(source: &str) -> Result<(), std::io::Error> {
    let source_path = Path::new(source);
    if !source_path.exists() {
        println!(
            "[MC-MANAGER] WARNING: Overrides directory {} not found. Skipping.",
            source
        );
        return Ok(());
    }
    println!("[MC-MANAGER] Syncing overrides from Nix store...");
    copy_recursive(source_path, Path::new("."))
}
