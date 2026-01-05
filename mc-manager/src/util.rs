use std::fs;
use std::path::Path;

pub fn copy_recursive(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    if source.is_dir() {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let dest = target.join(
                path.file_name()
                    .ok_or(std::io::Error::new(std::io::ErrorKind::Other, "Invalid filename"))?,
            );
            copy_recursive(&path, &dest)?;
        }
    } else {
        fs::copy(source, target)?;
    }
    Ok(())
}
