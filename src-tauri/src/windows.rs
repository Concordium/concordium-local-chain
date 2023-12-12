use regex::Regex;
use std::error::Error;

use std::fs;
use std::path::{Path, PathBuf};

pub fn find_concordium_node_executable() -> Result<PathBuf, Box<dyn Error>> {
    let paths = vec![Path::new(r"C:\Program Files\Concordium")];

    let pattern = Regex::new(r"Node \d+\.\d+\.\d+")?;

    for base_path in paths {
        if base_path.exists() && base_path.is_dir() {
            for entry in fs::read_dir(base_path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                        if pattern.is_match(dir_name) {
                            let executable_path = path.join("concordium-node.exe");
                            if executable_path.exists() {
                                return Ok(executable_path.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }

    Err("Concordium Node executable not found".into())
}
pub fn install_node_on_debian(destination_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    ///This is just a dummy function to make the code compile on windows due to the error in crate 'use std::os::unix::fs::PermissionsExt'
    unimplemented!("Not implemented for windows")
}
