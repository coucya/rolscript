use std::fs;
use std::path::Path;

use rolscript as rs;
use rolscript::Error as RError;
use rolscript::*;

pub struct StdLoader;
impl Loader for StdLoader {
    fn normalize_name(
        &mut self,
        requester: Ref<RModule>,
        name: Ref<RString>,
    ) -> Result<Ref<RString>, Error> {
        use std::path::*;
        let requester_path = Path::new(requester.normalized_name().as_str());
        let name_path = Path::new(name.as_str());

        fn normalize(path: &Path) -> Result<PathBuf, ()> {
            let mut normalized = PathBuf::new();
            for component in path.components() {
                match component {
                    Component::CurDir => (),
                    Component::ParentDir => {
                        if !normalized.pop() {
                            return Err(());
                        }
                    }
                    _ => normalized.push(component),
                }
            }
            Ok(normalized)
        }

        fn append_path(base: &mut PathBuf, path: &Path) {
            for component in path.components() {
                match component {
                    Component::Prefix(_) => (),
                    Component::RootDir => (),
                    Component::CurDir => (),
                    _ => base.push(component),
                }
            }
        }

        let mut normalized = PathBuf::new();

        if name_path.is_absolute() {
            append_path(&mut normalized, name_path);
        } else if name_path.is_relative() {
            normalized.push(requester_path);
            if normalized.pop() {
                append_path(&mut normalized, name_path);
            } else {
                return Err(runtime_error_fmt!(
                    "unable to normalize name: \"{}\"",
                    name.as_str()
                ));
            }
        } else {
            return Err(runtime_error_fmt!(
                "unable to normalize name: \"{}\"",
                name.as_str()
            ));
        }

        if let Ok(path) = normalized.canonicalize() {
            normalized = path;
        } else {
            if let Ok(p) = normalize(&normalized) {
                normalized = p;
            } else {
                return Err(runtime_error_fmt!(
                    "unable to normalize name: \"{}\"",
                    name.as_str()
                ));
            }
        }

        if let Some(normalized_path) = normalized.as_os_str().to_str() {
            let name = RString::new(normalized_path)?;
            Ok(name)
        } else {
            Err(runtime_error_fmt!(
                "unable to normalize name: \"{}\"",
                name.as_str()
            ))
        }
    }

    fn load(&mut self, normalized_name: Ref<RString>) -> Result<Ref<RFunction>, Error> {
        use std::fs;
        use std::path::*;
        let path = Path::new(normalized_name.as_str());
        if path.is_file() {
            if let Ok(s) = fs::read_to_string(path) {
                rs::parse_to_function(&s)
            } else {
                Err(runtime_error_fmt!(
                    "unable to load module: \"{}\"",
                    normalized_name.as_str()
                ))
            }
        } else {
            Err(runtime_error_fmt!(
                "not found module: \"{}\"",
                normalized_name.as_str()
            ))
        }
    }
}
