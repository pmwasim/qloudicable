//! The active tests' OS-independent parts.
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;
use qloudicable_core::bench::{self, DataTest};

const BLOCK: usize = 1 << 20;

/// Writes `megabytes` to a temporary file in `dir`, forcing it to disk so the OS
/// cache can't flatter the result, and always removes the file afterwards. Only a
/// new, uniquely named file is touched, so existing data is safe.
pub fn write_speed(dir: &Path, megabytes: u32, link_mbps: Option<u32>) -> io::Result<DataTest> {
    let path = dir.join(format!(".qloudicable-speedtest-{}.tmp", std::process::id()));
    let result = timed_write(&path, megabytes);
    let _ = fs::remove_file(&path);
    let seconds = result?;
    Ok(bench::data_test(megabytes as u64 * BLOCK as u64, seconds, link_mbps))
}

fn timed_write(path: &Path, megabytes: u32) -> io::Result<f64> {
    // create_new refuses to follow or overwrite anything already at this path.
    let mut f = File::options().write(true).create_new(true).open(path)?;
    // Varied bytes, so a compressing drive or filesystem can't shortcut the write.
    let mut block = vec![0u8; BLOCK];
    let mut x: u32 = 0x9E37_79B9;
    for b in block.iter_mut() {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *b = x as u8;
    }
    let start = Instant::now();
    for _ in 0..megabytes {
        f.write_all(&block)?;
    }
    f.sync_all()?;
    Ok(start.elapsed().as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_measures_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("qloudicable-speed-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let t = write_speed(&dir, 4, Some(5000)).unwrap();
        assert_eq!(t.bytes, 4 << 20);
        assert!(t.write_mbps > 0.0);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_folder_is_an_error() {
        assert!(write_speed(Path::new("/nonexistent/qloudicable"), 1, None).is_err());
    }
}
