// src/transaction/mod.rs

// 引入 StorageEngine
use crate::storage::StorageEngine;

pub struct TxnMgr {
    storage_engine: StorageEngine, // 添加 storage_engine 成员
    current_transaction_id: Option<u64>, // 用于跟踪当前事务
    pending_writes: std::collections::HashMap<String, Option<String>>, // 跟踪当前事务期间所做的更改
}

impl TxnMgr {
    pub fn new() -> Self {
        println!("TxnMgr initialized.");
        TxnMgr {
            storage_engine: StorageEngine::new(), // 初始化 storage_engine
            current_transaction_id: None,
            pending_writes: std::collections::HashMap::new(),
        }
    }

    pub fn begin_transaction(&mut self) -> u64 {
        // TODO: 实现更完善的事务 ID 生成逻辑
        let transaction_id = self.current_transaction_id.map_or(1, |id| id + 1);
        self.current_transaction_id = Some(transaction_id);
        self.pending_writes.clear(); // 清除任何旧的 pending_writes
        println!("TxnMgr: Beginning transaction ID: {}", transaction_id);
        transaction_id
    }

    // 这个方法可以用来代表事务中的一个操作
    pub fn execute_write_operation(&mut self, transaction_id: u64, key: String, value: String) {
        if self.current_transaction_id != Some(transaction_id) {
            // 或者可以返回一个错误
            println!("TxnMgr: Error - Operation on inactive or incorrect transaction ID: {}", transaction_id);
            return;
        }
        println!("TxnMgr: Executing write operation for transaction ID: {}. Key: {}, Value: {}", transaction_id, key, value);
        
        // 在实际写入 StorageEngine 之前，如果键已存在，则读取其当前值并将其存储在 pending_writes 中
        if !self.pending_writes.contains_key(&key) { // 只记录第一次修改前的值
            let original_value = self.storage_engine.read(&key);
            self.pending_writes.insert(key.clone(), original_value);
        }

        // 调用 self.storage_engine.write(key, value) 来实际存储数据
        self.storage_engine.write(key, value);
    }

    pub fn commit_transaction(&mut self, transaction_id: u64) {
        if self.current_transaction_id != Some(transaction_id) {
            println!("TxnMgr: Error - Attempting to commit inactive or incorrect transaction ID: {}", transaction_id);
            return;
        }
        println!("TxnMgr: Committing transaction ID: {}", transaction_id);
        self.pending_writes.clear(); // 成功提交后清除 pending_writes
        self.current_transaction_id = None; // 事务结束后重置
    }

    pub fn rollback_transaction(&mut self, transaction_id: u64) {
        if self.current_transaction_id != Some(transaction_id) {
            println!("TxnMgr: Error - Attempting to rollback inactive or incorrect transaction ID: {}", transaction_id);
            return;
        }
        println!("TxnMgr: Rolling back transaction ID: {}", transaction_id);
        for (key, original_value) in &self.pending_writes {
            match original_value {
                Some(val) => {
                    println!("TxnMgr: Rolling back key '{}' to value '{}'", key, val);
                    self.storage_engine.write(key.clone(), val.clone());
                }
                None => {
                    println!("TxnMgr: Rolling back by deleting key '{}'", key);
                    self.storage_engine.delete_data(key.clone()); // 假设 delete_data 存在于 StorageEngine
                }
            }
        }
        self.pending_writes.clear();
        self.current_transaction_id = None; // 事务结束后重置
    }

    // 实现 read_operation 方法
    pub fn read_operation(&self, transaction_id: u64, key: &String) -> Option<String> {
        // 简单的实现，直接读取，未来可以根据 transaction_id 实现更复杂的逻辑 (如MVCC)
        // 当前版本忽略 transaction_id，直接从 storage_engine 读取
        println!("TxnMgr: Reading key: '{}' for transaction ID: {}", key, transaction_id);
        self.storage_engine.read(key)
    }

    // 保留 delete 以便将来使用
    pub fn delete(&mut self, transaction_id: u64, key: String) {
        if self.current_transaction_id != Some(transaction_id) {
            println!("TxnMgr: Error - Operation on inactive or incorrect transaction ID: {}", transaction_id);
            return;
        }
        println!("TxnMgr: Deleting key: {} for transaction ID: {} (not fully implemented)", key, transaction_id);
        self.storage_engine.delete_data(key); // 调用 storage_engine 的删除
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_txn_write_and_read() {
        let mut tx_mgr = TxnMgr::new();
        let tx_id = tx_mgr.begin_transaction();
        let key = "txn_test_key".to_string();
        let value = "txn_test_value".to_string();

        tx_mgr.execute_write_operation(tx_id, key.clone(), value.clone());
        tx_mgr.commit_transaction(tx_id); // 需要提交事务才能保证写入

        // 重新开始一个事务来读取 (或者允许在同一事务内读取，取决于设计)
        let read_tx_id = tx_mgr.begin_transaction();
        assert_eq!(tx_mgr.read_operation(read_tx_id, &key), Some(value));
        tx_mgr.commit_transaction(read_tx_id);
    }
}