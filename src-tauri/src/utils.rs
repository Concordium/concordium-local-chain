use concordium_rust_sdk::common::types::{AccountAddress, Amount};
use concordium_rust_sdk::smart_contracts::types::InstanceInfo;
use concordium_rust_sdk::types::{AbsoluteBlockHeight, BlockItemSummary};
use std::collections::HashMap;
use tauri::Window;
use tokio::process::Child;

pub struct AppState {
    pub child_process: Option<Child>,
    pub main_window: Option<Window>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            child_process: None,
            main_window: None,
        }
    }
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct UiBlockInfo {
    pub hash: String,
    pub number: AbsoluteBlockHeight,
    pub amounts: HashMap<AccountAddress, Amount>,
    pub contracts: HashMap<String, InstanceInfo>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct TransactionsInfo {
    pub transactions: Vec<BlockItemSummary>,
}
