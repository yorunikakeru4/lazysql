use std::fs;
use std::path::Path;

use crate::themes::builtin::BUNDLED;

/// Seeds `~/.config/lazysql/themes/` with bundled themes on first run.
/// Does nothing if the directory already exists.
pub fn ensure_themes_dir(dir: &Path) {
    if dir.exists() {
        return;
    }
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    for (filename, content) in BUNDLED {
        let _ = fs::write(dir.join(filename), content);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn seeds_all_bundled_themes_when_dir_does_not_exist() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");

        ensure_themes_dir(&themes_dir);

        assert!(themes_dir.exists());
        for (filename, content) in BUNDLED {
            let path = themes_dir.join(filename);
            assert!(path.exists(), "{filename} should be seeded");
            let written = fs::read_to_string(&path).unwrap();
            assert_eq!(&written, content);
        }
    }

    #[test]
    fn does_not_touch_existing_themes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let themes_dir = tmp.path().join("themes");
        fs::create_dir_all(&themes_dir).unwrap();
        fs::write(themes_dir.join("custom.toml"), "sentinel").unwrap();

        ensure_themes_dir(&themes_dir);

        let content = fs::read_to_string(themes_dir.join("custom.toml")).unwrap();
        assert_eq!(content, "sentinel");
    }
}
