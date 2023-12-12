use crate::common::{
    create_next_chain_folder, download_file, json_to_toml, parse_block_info, transaction_info,
};
#[cfg(target_family = "unix")]
use crate::unix::{find_concordium_node_executable, install_node_on_debian};

#[cfg(not(target_family = "unix"))]
use crate::windows::{find_concordium_node_executable, install_node_on_debian};

use crate::AppState;
use tokio::task;

use dirs;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri::Window;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as AsyncCommand;

#[tauri::command]
pub async fn install(_handle: tauri::AppHandle) -> Result<(), String> {
    if find_concordium_node_executable().is_ok() {
        // Concordium Node is already installed, skip installation
        return Ok(());
    }

    // Detect the user's OS and architecture and generate the correct link for it.
    let download_url = if cfg!(target_os = "windows") {
        "https://distribution.concordium.software/windows/Signed/Node-6.0.4-0.msi"
    } else if cfg!(target_os = "macos") {
        "https://distribution.concordium.software/macos/signed/concordium-node-6.0.4-0.pkg"
    } else if cfg!(target_os = "linux") {
        "https://distribution.mainnet.concordium.software/deb/concordium-mainnet-node_6.0.4-0_amd64.deb"
    } else {
        return Err("Unsupported OS".into());
    };

    // Change Filename Depending on OS
    let file_name = if cfg!(target_os = "windows") {
        "concordium-node-lc1c.msi"
    } else if cfg!(target_os = "macos") {
        "concordium-node-lc1c.pkg"
    } else if cfg!(target_os = "linux") {
        "concordium-node-lc1c.deb"
    } else {
        return Err("Unsupported OS".into());
    };

    let downloads_folder = if cfg!(target_os = "linux") {
        PathBuf::from("/tmp")
    } else {
        dirs::download_dir()
            .unwrap_or_else(|| dirs::home_dir().expect("Failed to get home directory"))
    };
    let destination = downloads_folder.join(file_name);
    let destination_str = destination
        .to_str()
        .ok_or("Failed to convert path to string")?;

    match download_file(&download_url, &destination_str).await {
        Ok(_) => {
            if cfg!(target_os = "linux") {
                install_node_on_debian(&destination_str).map_err(|e| e.to_string())?;
                Ok(())
            } else if cfg!(target_os = "windows") {
                let status = Command::new("msiexec")
                    .args(&["/i", destination_str, "/passive", "/norestart"])
                    .status()
                    .map_err(|_| "Failed to execute msiexec command")?;

                if status.success() {
                    Ok(())
                } else {
                    Err("Installation failed. Ensure you are running with administrative privileges.".into())
                }
            } else if cfg!(target_os = "macos") {
                let status = Command::new("sudo")
                    .args(&["installer", "-pkg", destination_str, "-target", "/"])
                    .status()
                    .map_err(|_| "Failed to execute installer command")?;

                if status.success() {
                    Ok(())
                } else {
                    Err("Installation failed".into())
                }
            } else {
                Ok(())
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn verify_installation() -> Result<String, String> {
    let binary = find_concordium_node_executable().map_err(|e| e.to_string())?;

    let output = std::process::Command::new(&binary)
        .arg("--version")
        .output();

    match output {
        Ok(output) => {
            let stderr_string = String::from_utf8_lossy(&output.stderr).to_string();
            if stderr_string.contains("command not found")
                || stderr_string.contains("No such file or directory")
            {
                Err("Concordium node is not installed.".to_string())
            } else if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(stderr_string)
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn install_genesis_creator() -> Result<String, String> {
    std::env::set_var("CARGO_NET_GIT_FETCH_WITH_CLI", "true");

    if cfg!(target_os = "windows") {
        let genesis_installed = Command::new("genesis-creator")
            .args(&["--version"])
            .output();

        if let Err(_) = genesis_installed {
            if Command::new("cargo").arg("--version").output().is_err() {
                return Err("Cargo is not found in your PATH. Please install Rust and Cargo, and ensure they are in your PATH.".to_string());
            }

            let clone_output = Command::new("git")
                .args(&[
                    "clone",
                    "--recurse-submodules",
                    "https://github.com/Concordium/concordium-misc-tools.git",
                ])
                .output();

            if let Err(e) = clone_output {
                return Err(format!("Failed to clone the repository: {}", e));
            }

            // Install genesis-creator
            let install_output = Command::new("cargo")
                .args(&[
                    "install",
                    "--path",
                    "concordium-misc-tools/genesis-creator/",
                    "--locked",
                ])
                .output();

            if let Err(e) = install_output {
                return Err(format!("Failed to install with cargo: {}", e));
            }

            // Delete the cloned directory
            let delete_output = Command::new("cmd")
                .args(&["/C", "rmdir", "/s", "/q", "concordium-misc-tools"])
                .output();

            if let Err(e) = delete_output {
                return Err(format!("Failed to delete the directory: {}", e));
            }
        }
    } else {
        let home_dir = dirs::home_dir().ok_or("Unable to get home directory")?;
        let cargo_bin_path = home_dir.join(".cargo/bin/cargo").display().to_string();
        let output = std::process::Command::new(cargo_bin_path)
            .args(&[
                "install",
                "--git",
                "https://github.com/Concordium/concordium-misc-tools.git",
                "genesis-creator",
                "--locked",
            ])
            .output();

        match output {
            Ok(output_data) => {
                if output_data.status.success() {
                    return Ok(String::from_utf8_lossy(&output_data.stdout).to_string());
                } else {
                    return Err(String::from_utf8_lossy(&output_data.stderr).to_string());
                }
            }
            Err(e) => return Err(format!("Cargo install command failed: {}", e)),
        }
    }
    Ok("Successfully installed genesis-creator".to_string())
}

#[tauri::command]
pub async fn list_chain_folders() -> Result<Vec<String>, String> {
    let home_dir = dirs::home_dir().ok_or("Unable to get home directory")?;
    let folder_path = home_dir.join(".concordium-lc1c");
    let mut chain_folders = Vec::new();

    if folder_path.exists() {
        for entry in std::fs::read_dir(folder_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                let folder_name = path.file_name().unwrap().to_str().unwrap().to_string();
                if folder_name.starts_with("chain-") {
                    chain_folders.push(folder_name);
                }
            }
        }
    }

    chain_folders.sort_by(|a, b| {
        let a_num: i32 = a["chain-".len()..].parse().unwrap_or(0);
        let b_num: i32 = b["chain-".len()..].parse().unwrap_or(0);
        a_num.cmp(&b_num)
    });
    Ok(chain_folders)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LaunchMode {
    Easy,
    Advanced(String),
    Expert(String),
    FromExisting(String),
}
#[tauri::command]
pub async fn launch_template(
    app_state: tauri::State<'_, Arc<Mutex<AppState>>>,
    launch_mode: LaunchMode,
) -> Result<(), String> {
    // Retrieve the home directory
    let home_dir = dirs::home_dir().ok_or("Unable to get home directory")?;

    // Create a new folder within the home directory for LC1C if it does not exist
    let folder_path = home_dir.join(".concordium-lc1c");

    if !folder_path.exists() {
        std::fs::create_dir_all(&folder_path).map_err(|e| e.to_string())?;
    }

    let mut new_chain_folder: PathBuf = Default::default();
    let mut toml_path: PathBuf = Default::default();
    let mut should_run_concordium_node = false;

    match &launch_mode {
        LaunchMode::Easy => {
            new_chain_folder = create_next_chain_folder(&folder_path)?;
            let toml_url = "http://0x0.st/HpsT.toml";
            toml_path = new_chain_folder.join("desired_toml_file_name.toml");

            let toml_string = toml_path
                .to_str()
                .ok_or("Failed to convert path to string")?;

            let _ = match download_file(&toml_url, &toml_string).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            };
        }
        LaunchMode::Advanced(json_str) => {
            new_chain_folder = create_next_chain_folder(&folder_path)?;
            let json_value: JsonValue =
                serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
            let toml_value = json_to_toml(&json_value).ok_or("Failed to convert JSON to TOML")?;

            let toml_string = toml::to_string(&toml_value).map_err(|e| e.to_string())?;
            toml_path = new_chain_folder.join("desired_toml_file_name.toml");
            std::fs::write(&toml_path, &toml_string).map_err(|e| e.to_string())?;
        }
        LaunchMode::Expert(toml_str) => {
            new_chain_folder = create_next_chain_folder(&folder_path)?;
            toml_path = new_chain_folder.join("desired_toml_file_name.toml");
            std::fs::write(&toml_path, &toml_str).map_err(|e| e.to_string())?;
        }
        LaunchMode::FromExisting(folder_name) => {
            new_chain_folder = folder_path.join(folder_name);
            println!("Creating Genesis Creator!");
            should_run_concordium_node = true;
        }
    };

    if should_run_concordium_node {
        let binary = find_concordium_node_executable().map_err(|e| e.to_string())?;

        let mut child = AsyncCommand::new(binary)
            .args(&[
                "--no-bootstrap=true",
                "--listen-port",
                "8169",
                "--grpc2-listen-addr",
                "127.0.0.1",
                "--grpc2-listen-port",
                "20100",
                "--data-dir",
                ".",
                "--config-dir",
                ".",
                "--baker-credentials-file",
                "bakers/baker-0-credentials.json",
            ])
            .current_dir(&new_chain_folder)
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
            .expect("Failed to start the node.");

        let reader = BufReader::new(child.stderr.take().expect("Failed to capture stdout."));
        let mut state = app_state.lock().unwrap();
        state.child_process = Some(child);

        let window_clone: Option<Window> = state.main_window.clone();
        let mut lines: tokio::io::Lines<BufReader<tokio::process::ChildStderr>> = reader.lines();
        // Block Indexer
        tokio::spawn(async move {
            while let Some(line) = lines.next_line().await.expect("Failed to read line.") {
                // logging
                if let Some(window) = &window_clone {
                    println!("{:#?}", line);
                    //logging
                    if let Some(block_info) = parse_block_info().await {
                        window.emit("new-block", block_info).unwrap();
                    }
                }
            }
        });
        let window_clone: Option<Window> = state.main_window.clone();

        // TRANSACTION PROCESSOR
        tokio::spawn(async move {
            let mut latest_block;
            loop {
                match parse_block_info().await {
                    Some(block) => {
                        latest_block = block.number;
                        break;
                    }
                    None => {
                        eprintln!("Failed to fetch the block info. Retrying...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            }

            let original_latest_block = latest_block.clone();
            let mut latest_fetched: i64 = -1;

            loop {
                if let Some(window) = &window_clone {
                    println!("Processing transactions for block: {}", latest_block);

                    // Get all transactions for block
                    let transactions = transaction_info(latest_block).await.unwrap();
                    // Emit latest transactions as event
                    window.emit("transactions", transactions.clone()).unwrap();

                    if latest_fetched == -1 {
                        if latest_block.height > 0 {
                            latest_block.height -= 1;
                        } else {
                            // Once the initial descent is complete, update the state for subsequent loops.
                            let latest_new_block = parse_block_info().await.unwrap().number;
                            if latest_new_block.height <= original_latest_block.height {
                                break;
                            }
                            latest_block = latest_new_block;
                            latest_fetched = original_latest_block.height as i64;
                        }
                    } else if latest_fetched != -1 && latest_block.height as i64 > latest_fetched {
                        latest_block.height -= 1;
                    } else {
                        let latest_new_block = parse_block_info().await.unwrap().number;
                        if latest_new_block.height <= original_latest_block.height {
                            break;
                        }
                        latest_block = latest_new_block;
                        latest_fetched = original_latest_block.height as i64;
                    }
                }
            }
        });
    } else {
        let binary = if cfg!(target_os = "windows") {
            "genesis-creator".to_string()
        } else {
            home_dir
                .join(".cargo/bin/genesis-creator")
                .display()
                .to_string()
        };

        let output = std::process::Command::new(&binary)
            .args(&["generate", "--config", &toml_path.display().to_string()])
            .current_dir(&new_chain_folder) // No need to clone if you're only borrowing
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("Command executed successfully");
                    println!("Output: {}", String::from_utf8_lossy(&output.stdout));
                } else {
                    eprintln!(
                        "Command failed with error: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!("Error executing command: {}", e);
            }
        }

        // Finally call Concordium Node to Run the Local Chain but run it as an async command for the frontend to aknowledge
        // That it is actually running successfully.

        let binary = find_concordium_node_executable().map_err(|e| e.to_string())?;

        let mut child = AsyncCommand::new(binary)
            .args(&[
                "--no-bootstrap=true",
                "--listen-port",
                "8169",
                "--grpc2-listen-addr",
                "127.0.0.1",
                "--grpc2-listen-port",
                "20100",
                "--data-dir",
                ".",
                "--config-dir",
                ".",
                "--baker-credentials-file",
                "bakers/baker-0-credentials.json",
            ])
            .current_dir(&new_chain_folder)
            .stdout(std::process::Stdio::piped()) // Capture stdout
            .stderr(std::process::Stdio::piped()) // Capture stderr
            .spawn()
            .expect("Failed to start the node.");

        let reader = BufReader::new(child.stderr.take().expect("Failed to capture stdout."));
        let mut state = app_state.lock().unwrap();
        state.child_process = Some(child);

        let window_clone: Option<Window> = state.main_window.clone();
        let mut lines: tokio::io::Lines<BufReader<tokio::process::ChildStderr>> = reader.lines();
        // Block Indexer
        tokio::spawn(async move {
            while let Some(line) = lines.next_line().await.expect("Failed to read line.") {
                // logging
                if let Some(window) = &window_clone {
                    println!("{:#?}", line);
                    //logging
                    if let Some(block_info) = parse_block_info().await {
                        window.emit("new-block", block_info).unwrap();
                    }
                }
            }
        });
        let window_clone: Option<Window> = state.main_window.clone();

        // TRANSACTION PROCESSOR
        tokio::spawn(async move {
            let mut latest_block = parse_block_info().await.unwrap().number;
            let original_latest_block = latest_block.clone();
            let mut latest_fetched: i64 = -1; // Using a signed integer to handle -1 as uninitialized

            loop {
                if let Some(window) = &window_clone {
                    println!("Processing transactions for block: {}", latest_block);

                    // Get all transactions for block
                    let transactions = transaction_info(latest_block).await.unwrap();
                    // Emit latest transactions as event
                    window.emit("transactions", transactions.clone()).unwrap();

                    if latest_fetched == -1 {
                        if latest_block.height > 0 {
                            latest_block.height -= 1;
                        } else {
                            // Once the initial descent is complete, update the state for subsequent loops.
                            let latest_new_block = parse_block_info().await.unwrap().number;
                            if latest_new_block.height <= original_latest_block.height {
                                break;
                            }
                            latest_block = latest_new_block;
                            latest_fetched = original_latest_block.height as i64;
                        }
                    } else if latest_fetched != -1 && latest_block.height as i64 > latest_fetched {
                        latest_block.height -= 1;
                    } else {
                        let latest_new_block = parse_block_info().await.unwrap().number;
                        if latest_new_block.height <= original_latest_block.height {
                            break;
                        }
                        latest_block = latest_new_block;
                        latest_fetched = original_latest_block.height as i64;
                    }
                }
            }
        });
    }

    Ok(())
}

#[cfg(not(target_family = "unix"))]
#[tauri::command]
pub fn kill_chain(app_state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, String> {
    let output = Command::new("taskkill")
        .args(&["/F", "/IM", "concordium-node.exe"])
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        Ok("Killed concordium-node-collector process.".to_string())
    } else {
        Err("No running concordium-node-collector process to kill.".to_string())
    }
}

#[cfg(target_family = "unix")]
#[tauri::command]
pub async fn kill_chain(app_state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, String> {
    use nix::sys::signal::Signal;
    use nix::unistd::Pid;
    // Check if there's a child process to kill.
    let _child_to_kill = {
        let mut state = app_state.lock().unwrap();
        state.child_process.take() // This removes the child process from the state and gives us ownership.
    };
    {
        task::spawn_blocking(|| {
            let output = Command::new("pgrep")
                .arg("concordium-node")
                .output()
                .expect("Failed to execute command");

            if output.status.success() {
                let pid_str = String::from_utf8_lossy(&output.stdout);
                let pid: i32 = pid_str.trim().parse().expect("Failed to parse PID as i32");

                // Kill the process
                nix::sys::signal::kill(Pid::from_raw(pid), Signal::SIGKILL)
                    .expect("Failed to kill process");
                Ok("Killed concordium-node-collector process.".to_string())
            } else {
                Err("No running concordium-node-collector process to kill.".to_string())
            }
        })
        .await
        .unwrap()
    }
}
