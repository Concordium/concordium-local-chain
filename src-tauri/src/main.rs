// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use concordium_rust_sdk::smart_contracts::common::{AccountAddress, Amount};
use concordium_rust_sdk::types::smart_contracts::InstanceInfo;
use concordium_rust_sdk::types::{AbsoluteBlockHeight, BlockItemSummary};
use concordium_rust_sdk::v2::{self, AccountIdentifier};
use concordium_rust_sdk::{endpoints::Endpoint, types::hashes::BlockHash};
use dirs;
use futures::StreamExt;
#[cfg(not(target_os = "windows"))]
use nix::sys::signal::Signal;
#[cfg(not(target_os = "windows"))]
use nix::unistd::Pid;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri::{Manager, Window};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::process::Command as AsyncCommand;
use tokio::task;
use tokio::time::Duration;
use toml::Value as TomlValue;
/* ---------------------------------------------------- MUTEX APP STATE ------------------------------------------------------------ */

struct AppState {
    child_process: Option<Child>,
    main_window: Option<Window>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            child_process: None,
            main_window: None,
        }
    }
}

/* ---------------------------------------------------- INSTALL COMMAND ------------------------------------------------------------ */

fn find_concordium_node_executable() -> Result<PathBuf, Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    let paths = vec![(
        Path::new(r"C:\Program Files\Concordium"),
        Some(Regex::new(r"Node \d+\.\d+\.\d+")?),
    )];

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let paths = vec![
        (Path::new("/usr/bin/concordium-node"), None::<Regex>),
        (Path::new("/usr/local/bin/concordium-node"), None),
    ];

    for (base_path, pattern_opt) in paths {
        if let Some(pattern) = pattern_opt {
            if base_path.exists() && base_path.is_dir() {
                for entry in fs::read_dir(base_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                            if pattern.is_match(dir_name) {
                                let executable_path = path.join("concordium-node.exe");
                                if executable_path.exists() {
                                    return Ok(executable_path);
                                }
                            }
                        }
                    }
                }
            }
        } else if base_path.exists() {
            return Ok(base_path.to_path_buf());
        }
    }

    Err("Concordium Node executable not found".into())
}

#[tauri::command]
async fn install(handle: tauri::AppHandle) -> Result<(), String> {
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
                let script_str = handle
                    .path_resolver()
                    .resolve_resource("assets/install_concordium_debian.sh")
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                let status = Command::new("pkexec")
                    .args(&["bash", &script_str, destination_str])
                    .status()
                    .map_err(|_| "Failed to execute the bash script with pkexec")?;

                if status.success() {
                    Ok(())
                } else {
                    Err("Installation failed".into())
                }
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

/* ---------------------------------------------------- Verify Installation COMMAND ------------------------------------------------------------ */

#[tauri::command]
async fn verify_installation() -> Result<String, String> {
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

/* ---------------------------------------------------- INSTALL Genesis COMMAND ------------------------------------------------------------ */

#[tauri::command]
fn install_genesis_creator() -> Result<String, String> {
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

async fn download_file(url: &str, destination: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Make a GET request to the URL
    let response = reqwest::get(url).await?;

    // Check if the request was successful
    if response.status() != reqwest::StatusCode::OK {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "Failed to download file. HTTP response: {}",
                response.status()
            ),
        )));
    }

    // Create a new file at the destination path
    let mut dest_file = File::create(destination)?;

    // Get the entire response body as Bytes
    let content = response.bytes().await?;

    // Write the content to the file
    dest_file.write_all(&content)?;

    let _metadata = dest_file.metadata()?;
    println!("Downloaded {} bytes to {}", _metadata.len(), destination);

    // if mac OS open the file after downloading
    if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg(destination)
            .output()?;
    }

    // TODO: Add for other OS as well
    Ok(())
}
#[tauri::command]
async fn list_chain_folders() -> Result<Vec<String>, String> {
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

/* ---------------------------------------------------- TEMPLATE LAUNCH COMMAND ------------------------------------------------------------ */

fn json_to_toml(json_value: &JsonValue) -> Option<TomlValue> {
    match json_value {
        JsonValue::Null => Some(TomlValue::String("".to_string())),
        JsonValue::Bool(b) => Some(TomlValue::Boolean(*b)),
        JsonValue::Number(n) => {
            if n.is_f64() {
                Some(TomlValue::Float(n.as_f64().unwrap()))
            } else if n.is_i64() {
                Some(TomlValue::Integer(n.as_i64().unwrap()))
            } else {
                None
            }
        }
        JsonValue::String(s) => Some(TomlValue::String(s.clone())),
        JsonValue::Array(arr) => {
            let toml_arr: Vec<TomlValue> = arr.iter().filter_map(|v| json_to_toml(v)).collect();
            Some(TomlValue::Array(toml_arr))
        }
        JsonValue::Object(obj) => {
            let mut toml_table = toml::value::Table::new();
            for (k, v) in obj.iter() {
                if let Some(toml_v) = json_to_toml(v) {
                    toml_table.insert(k.clone(), toml_v);
                }
            }
            Some(TomlValue::Table(toml_table))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum LaunchMode {
    Easy,
    Advanced(String),
    Expert(String),
    FromExisting(String),
}
#[tauri::command]
async fn launch_template(
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
            // loop {
            //     if let Some(window) = &window_clone {
            //         if let Some(block_info) = parse_block_info().await {
            //             // Check if the block is not the same as the last one
            //             window.emit("new-block", block_info.clone()).unwrap();
            //         }
            //     }

            //     tokio::time::sleep(Duration::from_millis(100)).await; // Optional: avoid busy waiting by adding a small sleep
            // }
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
            // BLOCK INDEXER
            // tokio::spawn(async move {
            //     loop {
            //         if let Some(window) = &window_clone {
            //             if let Some(block_info) = parse_block_info().await {
            //                 // Check if the block is not the same as the last one
            //                 window.emit("new-block", block_info.clone()).unwrap();
            //             }
            //         }

            //         tokio::time::sleep(Duration::from_millis(100)).await; // Optional: avoid busy waiting by adding a small sleep
            //     }
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

async fn parse_block_info() -> Option<UiBlockInfo> {
    let (block_hash, number) = account_info()
        .await
        .map_err(|e| {
            eprintln!("Error fetching account info: {}", e);
            e
        })
        .ok()?;

    let amounts_map = amount_info(block_hash)
        .await
        .map_err(|e| {
            eprintln!("Error fetching amount info: {}", e);
            e
        })
        .ok()?;

    let contracts_map = instance_list(block_hash)
        .await
        .map_err(|e| {
            eprintln!("Error fetching instance list: {}", e);
            e
        })
        .ok()?;

    Some(UiBlockInfo {
        hash: block_hash.to_string(),
        number,
        amounts: amounts_map,
        contracts: contracts_map,
    })
}

fn create_next_chain_folder(base_path: &Path) -> Result<PathBuf, String> {
    let mut counter = 1;

    loop {
        let folder_name = format!("chain-{}", counter);
        let folder_path = base_path.join(&folder_name);

        if !folder_path.exists() {
            // Create the folder and then return its path
            std::fs::create_dir(&folder_path).map_err(|e| e.to_string())?;
            return Ok(folder_path);
        }

        counter += 1;
    }
}

/* ---------------------------------------------------- KILL CHAIN COMMAND ------------------------------------------------------------------------ */
#[tauri::command]
async fn kill_chain(app_state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, String> {
    // Check if there's a child process to kill.
    let _child_to_kill = {
        let mut state = app_state.lock().unwrap();
        state.child_process.take() // This removes the child process from the state and gives us ownership.
    };

    #[cfg(target_os = "windows")]
    {
        // Windows implementation using taskkill command
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
    #[cfg(not(target_os = "windows"))]
    {
        // Unix-like systems implementation using pgrep and kill
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

/* ---------------------------------------------------- SUBTOOLS --------------------------------------------------------------------------- */

#[derive(Debug, serde::Serialize, Clone)]
struct UiBlockInfo {
    hash: String,
    number: AbsoluteBlockHeight,
    amounts: HashMap<AccountAddress, Amount>,
    contracts: HashMap<String, InstanceInfo>,
}
#[derive(Debug, serde::Serialize, Clone)]
struct TransactionsInfo {
    transactions: Vec<BlockItemSummary>,
}
async fn instance_list(hash: BlockHash) -> anyhow::Result<HashMap<String, InstanceInfo>> {
    println!("Fetching instance list for block hash: {:?}", hash);

    let endpoint_node = Endpoint::from_str("http://127.0.0.1:20100")?;
    let mut client = v2::Client::new(endpoint_node).await?;
    let mut contracts = client.get_instance_list(&hash).await?;

    let mut amounts_map = HashMap::new();
    while let Some(a) = contracts.response.next().await {
        match a {
            Ok(contract_addr) => {
                println!("Fetching info for contract address: {:?}", contract_addr);

                let info = client.get_instance_info(contract_addr, &hash).await?;

                let key_string = contract_addr.index.to_string();
                let contract_info = info.response;
                amounts_map.insert(key_string, contract_info);

                println!(
                    "Successfully fetched info for contract address: {:?}",
                    contract_addr
                );
            }
            Err(e) => {
                println!(
                    "Failed to get contract address for block hash {}: {}",
                    hash, e
                );
                return Err(anyhow::anyhow!("Failed to get contract address: {}", e));
            }
        }
    }

    if amounts_map.is_empty() {
        println!("No contract instances found for the given block hash.");
    } else {
        println!(
            "Total contract instances fetched for block hash {}: {}",
            hash,
            amounts_map.len()
        );
    }

    println!("Finished fetching instance list for block hash: {:?}", hash);

    Ok(amounts_map)
}

async fn transaction_info(number: AbsoluteBlockHeight) -> anyhow::Result<TransactionsInfo> {
    println!("Fetching transaction info for block height: {:?}", number);

    let endpoint_node = Endpoint::from_str("http://127.0.0.1:20100")?;
    let mut client = v2::Client::new(endpoint_node).await?;
    let res = client.get_block_transaction_events(&number).await?;
    let mut summaries: Vec<BlockItemSummary> = Vec::new(); // Created locally

    let mut transactions = res.response;
    while let Some(item) = transactions.next().await {
        match item {
            Ok(summary) => {
                println!("Successfully fetched transaction: {:?}", summary);
                summaries.push(summary);
            }
            Err(e) => {
                println!(
                    "Error fetching transaction for block height {}: {}",
                    number, e
                );
                return Err(anyhow::anyhow!("Error fetching transaction event: {}", e));
            }
        }
    }

    if summaries.is_empty() {
        println!(
            "No transactions found for the given block height {}.",
            number
        );
    } else {
        println!(
            "Total transactions fetched for block height {}: {}",
            number,
            summaries.len()
        );
    }

    println!(
        "Finished fetching transaction info for block height: {:?}",
        number
    );

    Ok(TransactionsInfo {
        transactions: summaries.to_vec(),
    })
}

async fn amount_info(hash: BlockHash) -> anyhow::Result<HashMap<AccountAddress, Amount>> {
    println!("Fetching amount info for block hash: {:?}", hash);

    let endpoint_node = Endpoint::from_str("http://127.0.0.1:20100")?;
    let mut client = v2::Client::new(endpoint_node).await?;
    let mut accounts = client.get_account_list(&hash).await?;

    let mut amounts_map = HashMap::new();
    while let Some(a) = accounts.response.next().await {
        match a {
            Ok(account_addr) => {
                let addr = AccountIdentifier::Address(account_addr.clone());
                let info = client.get_account_info(&addr, hash).await?;
                let amt = info.response.account_amount;
                amounts_map.insert(a?, amt);
            }
            Err(e) => {
                println!("Failed to get account address. Error: {:?}", e);
                return Err(anyhow::anyhow!("Failed to get account address: {}", e));
            }
        }
    }

    Ok(amounts_map)
}

async fn account_info() -> anyhow::Result<(BlockHash, AbsoluteBlockHeight)> {
    let endpoint_node = Endpoint::from_str("http://127.0.0.1:20100").map_err(|e| {
        println!("[ERROR] Failed to create endpoint: {:?}", e);
        e
    })?;

    println!("[INFO] Endpoint created successfully.");

    let mut client = v2::Client::new(endpoint_node).await.map_err(|e| {
        println!("[ERROR] Failed to create client: {:?}", e);
        e
    })?;

    println!("[INFO] Client created successfully.");

    let mut accounts = client
        .get_account_list(&v2::BlockIdentifier::LastFinal)
        .await
        .map_err(|e| {
            println!("[ERROR] Failed to get account list: {:?}", e);
            e
        })?;

    println!("[INFO] Account list retrieved successfully.");

    let block = accounts.block_hash;
    let mut account_addr: Option<AccountAddress> = None;
    while let Some(a) = accounts.response.next().await {
        account_addr = a.ok();
    }

    if account_addr.is_none() {
        println!("[WARN] No account address found.");
        return Err(anyhow::anyhow!("No account address found"));
    }

    let addr = AccountIdentifier::Address(account_addr.unwrap());

    let info = client.get_account_info(&addr, block).await.map_err(|e| {
        println!("[ERROR] Failed to get account info: {:?}", e);
        e
    })?;

    println!("[INFO] Account info retrieved successfully.");

    let hash = info.block_hash;

    let blocks = client.get_block_info(&block).await.map_err(|e| {
        println!("[ERROR] Failed to get block info: {:?}", e);
        e
    })?;

    println!("[INFO] Block info retrieved successfully.");

    let block_number = blocks.response.block_height;

    Ok((hash, block_number))
}

fn main() {
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tauri::Builder::default()
        .manage(app_state.clone())
        .setup(move |app| {
            // Get a reference to the main window
            let main_window = app.get_window("main").unwrap();

            // Store the window reference in the app state
            let mut state = app_state.lock().unwrap();
            state.main_window = Some(main_window);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            install,
            verify_installation,
            install_genesis_creator,
            launch_template,
            list_chain_folders,
            kill_chain
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}