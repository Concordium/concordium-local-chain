use ar;
use dirs;
use lzma_rs;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::fs::Permissions;
use std::io::{self, Cursor, Read};

#[cfg(target_family = "unix")]
use std::os::unix::fs::PermissionsExt;
/// Target family has been set so that the code compiles on windows without error
use std::path::{Path, PathBuf};
use tar::Archive;

///  Function to install concordium-node binary at $HOME/.concordium-lc1c/bin
/// for debian based linux distrbution or other supported distributions
pub fn install_node_on_debian(destination_str: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut dest_path = dirs::home_dir().unwrap();
    let concordium_dir = dest_path.join(".concordium-lc1c");
    let bin_dir = concordium_dir.join("bin");
    println!("{:?}", destination_str);
    match fs::create_dir_all(&bin_dir) {
        Ok(_) => println!("Directory created successfully at {:?}", bin_dir),
        Err(e) => println!("Error creating directory: {}", e),
    }
    dest_path.push(".concordium-lc1c/bin/concordium-node");
    let mut file = File::open(destination_str).map_err(|e| format!("Error opening file: {}", e))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| format!("Error reading file: {}", e))?;

    let mut ar = ar::Archive::new(Cursor::new(data));
    while let Some(entry_result) = ar.next_entry() {
        let entry = entry_result?;
        if entry.header().identifier() == b"data.tar.xz" {
            let mut archive = Vec::new();
            lzma_rs::xz_decompress(&mut std::io::BufReader::new(entry), &mut archive)?;

            let mut archive = Archive::new(Cursor::new(archive));
            for file in archive.entries()? {
                let mut file = file?;
                if is_target_file(&file.path()?) {
                    let mut out_file = File::create(&dest_path)?;
                    io::copy(&mut file, &mut out_file)?;
                    // Only compiled for linux
                    #[cfg(target_os = "linux")]
                    {
                        out_file.set_permissions(Permissions::from_mode(0o755))?;
                    }

                    break;
                }
            }
            break;
        }
    }
    Ok(())
}
/// check if the target file exists or not
fn is_target_file(path: &std::path::Path) -> bool {
    path.display()
        .to_string()
        .starts_with("./usr/bin/concordium-mainnet-node")
}
pub fn find_concordium_node_executable() -> Result<PathBuf, Box<dyn Error>> {
    let home_dir = dirs::home_dir().expect("Could not find home directory");
    let local_bin_path = home_dir.join(".concordium-lc1c/bin/concordium-node");
    let paths = vec![
        Path::new("/usr/bin/concordium-node"),
        local_bin_path.as_path(),
        Path::new("/usr/local/bin/concordium-node"),
    ];

    for base_path in paths {
        if base_path.exists() && base_path.is_file() {
            return Ok(base_path.to_path_buf());
        }
    }

    Err("Concordium Node executable not found".into())
}
