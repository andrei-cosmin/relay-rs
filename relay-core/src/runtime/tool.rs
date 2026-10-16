use std::{env, path::PathBuf};

pub struct Tool(pub &'static str);

impl Tool {
    pub fn program(&self) -> PathBuf {
        let bundled = env::current_exe()
            .ok()
            .and_then(|executable| executable.parent().map(|folder| folder.join(self.0)));
        match bundled {
            Some(path) if path.is_file() => path,
            _ => PathBuf::from(self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn a_tool_next_to_the_executable_is_used_by_its_full_path() {
        let folder = env::current_exe().unwrap().parent().unwrap().to_path_buf();
        let path = folder.join("relay-core-test-bundled-tool");
        fs::write(&path, "").unwrap();
        let program = Tool("relay-core-test-bundled-tool").program();
        fs::remove_file(&path).unwrap();
        assert_eq!(program, path);
    }

    #[test]
    fn a_tool_missing_next_to_the_executable_is_looked_up_on_the_path() {
        assert_eq!(
            Tool("relay-core-test-missing-tool").program(),
            PathBuf::from("relay-core-test-missing-tool")
        );
    }
}
