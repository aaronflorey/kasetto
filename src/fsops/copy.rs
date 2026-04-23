use std::fs;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use crate::error::Result;

pub(crate) fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    fs::create_dir_all(dst)?;
    copy_dir_contents(src, dst)
}

fn copy_dir_contents(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let target = dst.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            let resolved = fs::canonicalize(&src_path)?;
            let meta = fs::metadata(&resolved)?;
            if meta.is_dir() {
                fs::create_dir_all(&target)?;
                copy_dir_contents(&resolved, &target)?;
            } else {
                copy_file(&resolved, &target)?;
            }
        } else if file_type.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir_contents(&src_path, &target)?;
        } else {
            copy_file(&src_path, &target)?;
        }
    }
    Ok(())
}

fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let reader = BufReader::new(fs::File::open(src)?);
    let mut writer = BufWriter::new(fs::File::create(dst)?);
    let mut buf_reader = reader;
    std::io::copy(&mut buf_reader, &mut writer)?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()))
    }

    #[cfg(unix)]
    #[test]
    fn copy_dir_follows_symlinked_directories() {
        use std::os::unix::fs::symlink;

        let src = temp_dir("kasetto-copy-src");
        let refs_dir = src.join("references");
        fs::create_dir_all(&refs_dir).expect("create refs");
        fs::write(refs_dir.join("guide.md"), "hello").expect("write file");
        symlink("references", src.join("linked-references")).expect("create symlink");

        let dst = temp_dir("kasetto-copy-dst");
        copy_dir(&src, &dst).expect("copy dir");

        assert!(dst.join("linked-references/guide.md").is_file());
        assert!(dst.join("references/guide.md").is_file());

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dst);
    }
}
