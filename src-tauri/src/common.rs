use crate::utils::{TransactionsInfo, UiBlockInfo};
use concordium_rust_sdk::smart_contracts::common::{AccountAddress, Amount};
use concordium_rust_sdk::smart_contracts::types::InstanceInfo;
use concordium_rust_sdk::types::{AbsoluteBlockHeight, BlockItemSummary};
use concordium_rust_sdk::v2::{self, AccountIdentifier};
use concordium_rust_sdk::{endpoints::Endpoint, types::hashes::BlockHash};
use futures::stream::StreamExt;
use reqwest;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use toml::Value as TomlValue;

pub async fn download_file(url: &str, destination: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

pub async fn parse_block_info() -> Option<UiBlockInfo> {
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

pub fn create_next_chain_folder(base_path: &Path) -> Result<PathBuf, String> {
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

pub async fn instance_list(hash: BlockHash) -> anyhow::Result<HashMap<String, InstanceInfo>> {
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

pub async fn transaction_info(number: AbsoluteBlockHeight) -> anyhow::Result<TransactionsInfo> {
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

pub async fn amount_info(hash: BlockHash) -> anyhow::Result<HashMap<AccountAddress, Amount>> {
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

pub async fn account_info() -> anyhow::Result<(BlockHash, AbsoluteBlockHeight)> {
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

pub fn json_to_toml(json_value: &JsonValue) -> Option<TomlValue> {
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
