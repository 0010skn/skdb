// src/storage/mod.rs
use std::collections::HashMap;

#[derive(Clone)] // 添加 Clone trait 以支持 snapshot
pub struct StorageEngine {
    data: HashMap<String, String>,
    versions: HashMap<String, Vec<String>>, // 用于存储每个键的历史版本
}

impl StorageEngine {
    pub fn new() -> Self {
        println!("StorageEngine initialized.");
        StorageEngine {
            data: HashMap::new(),
            versions: HashMap::new(),
        }
    }

    pub fn write(&mut self, key: String, value: String) {
        println!("StorageEngine: Writing data for key: '{}', value: '{}'", key, value);
        // 在写入新值时，将旧值（如果存在）保存到历史版本中
        if let Some(old_value) = self.data.get(&key) {
            self.versions.entry(key.clone()).or_insert_with(Vec::new).push(old_value.clone());
        }
        self.data.insert(key, value);
    }

    pub fn read(&self, key: &String) -> Option<String> {
        println!("StorageEngine: Reading data for key: '{}'", key);
        self.data.get(key).cloned()
    }

    // 获取指定键的特定历史版本
    pub fn get_version(&self, key: &String, version_index: usize) -> Option<String> {
        println!("StorageEngine: Getting version {} for key: '{}'", version_index, key);
        // 确保从 versions.get(key) 返回的 Vec<String> 中正确索引并克隆值
        self.versions.get(key).and_then(|versions| {
            versions.get(version_index).cloned()
        })
    }

    // 创建当前存储状态的快照
    pub fn snapshot(&self) -> Self {
        println!("StorageEngine: Creating snapshot...");
        self.clone()
    }

    // 回滚到指定的快照状态
    pub fn rollback_to_snapshot(&mut self, snapshot: Self) {
        println!("StorageEngine: Rolling back to snapshot...");
        self.data = snapshot.data;
        self.versions = snapshot.versions;
    }

    // 实现 delete 方法
    pub fn delete(&mut self, key: &String) -> Option<String> {
        println!("StorageEngine: Deleting data for key: '{}'", key);
        // 为了简单起见，暂时只从 data 中删除
        // 未来可以考虑如何处理其在 versions 中的历史
        // 例如，可以保留历史版本或添加一个特殊的“已删除”标记。
        // 如果需要将删除也视为一个版本，可以在删除前将当前值存入 versions
        // if let Some(old_value) = self.data.get(key) {
        //     self.versions.entry(key.clone()).or_insert_with(Vec::new).push(old_value.clone());
        // }
        self.data.remove(key)
    }

    // 用于事务回滚时，如果键是新创建的，则需要删除
    // 这个方法与上面的 delete 类似，但接受 String 而不是 &String，并且不特意处理版本历史
    // TxnMgr 将使用此方法
    pub fn delete_data(&mut self, key: String) -> Option<String> {
        println!("StorageEngine: (Internal) Deleting data for key: '{}'", key);
        self.data.remove(&key)
    }

    pub fn load_from_disk() {
        // TODO: 实现从磁盘加载数据逻辑
        println!("StorageEngine: Loading data from disk (Not yet implemented)");
    }

    pub fn flush_to_disk(&self) {
        // TODO: 实现将内存中的更改刷新到磁盘逻辑
        println!("StorageEngine: Flushing data to disk (Not yet implemented)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_read() {
        let mut engine = StorageEngine::new();
        let key = "test_key".to_string();
        let value = "test_value".to_string();
        engine.write(key.clone(), value.clone());
        assert_eq!(engine.read(&key), Some(value));
    }

    #[test]
    fn test_versioning() {
        let mut engine = StorageEngine::new();
        let key = "version_key".to_string();
        let value1 = "value1".to_string();
        let value2 = "value2".to_string();

        engine.write(key.clone(), value1.clone());
        engine.write(key.clone(), value2.clone());

        assert_eq!(engine.read(&key), Some(value2));
        assert_eq!(engine.get_version(&key, 0), Some(value1));
        assert_eq!(engine.get_version(&key, 1), None); // 只有1个历史版本 (value1)
    }

    #[test]
    fn test_snapshot_and_rollback() {
        let mut engine = StorageEngine::new();
        let key1 = "key1".to_string();
        let value1 = "value1".to_string();
        engine.write(key1.clone(), value1.clone());

        let snapshot = engine.snapshot(); // 创建快照

        let key2 = "key2".to_string();
        let value2 = "value2".to_string();
        engine.write(key2.clone(), value2.clone()); // 修改引擎状态

        assert_eq!(engine.read(&key1), Some(value1.clone()));
        assert_eq!(engine.read(&key2), Some(value2.clone()));

        engine.rollback_to_snapshot(snapshot); // 回滚

        assert_eq!(engine.read(&key1), Some(value1));
        assert_eq!(engine.read(&key2), None); // key2 在快照之后添加，回滚后应不存在
    }
}