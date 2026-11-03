//! Opt-in startup for the current Windows account. The registry is the source
//! of truth; reading a setting never creates or changes a startup entry.

use crate::error::{Error, Result};

#[cfg(windows)]
mod platform {
    use std::ffi::OsString;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};
    use winreg::RegKey;

    use super::{Error, Result};

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "Jotter";

    fn launch_command(exe: &Path) -> Result<OsString> {
        let mut command = OsString::from("\"");
        command.push(exe.as_os_str());
        command.push("\"");
        // Windows documents a 260-character limit for a Run command line.
        if command.encode_wide().count() > 260 {
            return Err(Error::Invalid("程序路径过长，请移到较短的路径后开启自启".into()));
        }
        Ok(command)
    }

    fn read_entry(root: &RegKey, key_path: &str) -> io::Result<Option<OsString>> {
        let result = root
            .open_subkey_with_flags(key_path, KEY_QUERY_VALUE)
            .and_then(|key| key.get_value(VALUE_NAME));
        match result {
            Ok(command) => Ok(Some(command)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn write_entry(root: &RegKey, key_path: &str, command: Option<&OsString>) -> io::Result<()> {
        if let Some(command) = command {
            let (key, _) = root.create_subkey_with_flags(key_path, KEY_SET_VALUE)?;
            key.set_value(VALUE_NAME, command)
        } else {
            let result = root
                .open_subkey_with_flags(key_path, KEY_SET_VALUE)
                .and_then(|key| key.delete_value(VALUE_NAME));
            match result {
                Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
                result => result,
            }
        }
    }

    pub fn enabled() -> Result<bool> {
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let Some(registered) = read_entry(&root, RUN_KEY)? else {
            return Ok(false);
        };
        Ok(registered == launch_command(&std::env::current_exe()?)?)
    }

    pub fn set_enabled(enabled: bool) -> Result<()> {
        let command = if enabled {
            Some(launch_command(&std::env::current_exe()?)?)
        } else {
            None
        };
        write_entry(&RegKey::predef(HKEY_CURRENT_USER), RUN_KEY, command.as_ref())?;
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn quotes_unicode_paths_with_spaces_and_rejects_overlong_commands() {
            assert_eq!(
                launch_command(Path::new(r"C:\我的便签\Pet Notes\Jotter.exe")).unwrap(),
                OsString::from(r#""C:\我的便签\Pet Notes\Jotter.exe""#)
            );
            assert!(launch_command(Path::new(&"a".repeat(259))).is_err());
        }

        #[test]
        fn registry_round_trip_preserves_other_values() {
            // Never use the actual Run key in tests. This isolated key cannot
            // launch anything at sign-in, even if the test process is killed.
            let root = RegKey::predef(HKEY_CURRENT_USER);
            let path = format!(r"Software\JotterStartupTest-{}", uuid::Uuid::new_v4());
            struct Cleanup(String);
            impl Drop for Cleanup {
                fn drop(&mut self) {
                    let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
                }
            }
            let _cleanup = Cleanup(path.clone());

            assert_eq!(read_entry(&root, &path).unwrap(), None);
            write_entry(&root, &path, None).unwrap();
            assert!(root.open_subkey(&path).is_err());

            let old = launch_command(Path::new(r"C:\旧版本\Jotter.exe")).unwrap();
            let new = launch_command(Path::new(r"C:\新位置\Pet Notes\Jotter.exe")).unwrap();
            write_entry(&root, &path, Some(&old)).unwrap();
            assert_eq!(read_entry(&root, &path).unwrap(), Some(old));

            let key = root.open_subkey_with_flags(&path, KEY_SET_VALUE).unwrap();
            key.set_value("OtherApp", &"keep").unwrap();
            drop(key);
            write_entry(&root, &path, Some(&new)).unwrap();
            assert_eq!(read_entry(&root, &path).unwrap(), Some(new));
            write_entry(&root, &path, None).unwrap();
            write_entry(&root, &path, None).unwrap();
            assert_eq!(read_entry(&root, &path).unwrap(), None);
            let key = root.open_subkey(&path).unwrap();
            assert_eq!(key.get_value::<String, _>("OtherApp").unwrap(), "keep");
        }
    }
}

#[cfg(windows)]
pub use platform::{enabled, set_enabled};

#[cfg(not(windows))]
pub fn enabled() -> Result<bool> {
    Err(Error::Invalid("开机自启目前仅支持 Windows".into()))
}

#[cfg(not(windows))]
pub fn set_enabled(_enabled: bool) -> Result<()> {
    Err(Error::Invalid("开机自启目前仅支持 Windows".into()))
}
