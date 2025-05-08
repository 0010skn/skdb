// 引用项目根目录下的 lib.rs 中定义的模块
// 假设你的项目名称是 skdb (根据 Cargo.toml 中的 name)
// 如果不是，请替换为正确的项目名称
use skdb::api::ApiMgr;

fn main() {
    println!("你好，FeatherScaleDB!");

    // 创建 ApiMgr 实例
    let mut api_mgr = ApiMgr::new();

    println!("\n--- 演示事务成功提交 ---");
    let txn_id_commit = api_mgr.begin_transaction();
    println!("主程序：已启动事务 {}", txn_id_commit);

    let key_c1 = "commit_key_1".to_string();
    let value_c1 = "value1_to_be_committed".to_string();
    let key_c2 = "commit_key_2".to_string();
    let value_c2 = "value2_to_be_committed".to_string();

    api_mgr.put(txn_id_commit, key_c1.clone(), value_c1.clone());
    println!("主程序：事务 {} - 放入键 '{}'，值 '{}'", txn_id_commit, key_c1, value_c1);
    api_mgr.put(txn_id_commit, key_c2.clone(), value_c2.clone());
    println!("主程序：事务 {} - 放入键 '{}'，值 '{}'", txn_id_commit, key_c2, value_c2);

    api_mgr.commit_transaction(txn_id_commit);
    println!("主程序：事务 {} 已提交。", txn_id_commit);

    // 验证数据是否已提交
    println!("\n主程序：尝试在提交后获取键：");
    match api_mgr.get(key_c1.clone()) {
        Some(value) => {
            println!("主程序：提交后获取 - 键 '{}' 的值：'{}'", key_c1, value);
            assert_eq!(value, value_c1, "键 key_c1 的值应该是已提交的值");
        }
        None => println!("主程序：提交后获取 - 未找到键 '{}' 的值。这是预料之外的。", key_c1),
    }
    match api_mgr.get(key_c2.clone()) {
        Some(value) => {
            println!("主程序：提交后获取 - 键 '{}' 的值：'{}'", key_c2, value);
            assert_eq!(value, value_c2, "键 key_c2 的值应该是已提交的值");
        }
        None => println!("主程序：提交后获取 - 未找到键 '{}' 的值。这是预料之外的。", key_c2),
    }

    println!("\n--- 演示事务回滚 (修改现有键并添加新键) ---");
    let key_rb_existing = "rollback_key_existing".to_string();
    let value_rb_initial = "initial_value_for_rollback_existing".to_string();
    let value_rb_txn_modified = "modified_value_in_txn".to_string();
    let key_rb_new = "rollback_key_new_in_txn".to_string();
    let value_rb_new = "new_value_in_txn".to_string();

    // 1. 先放入一个初始值并提交
    let setup_txn_rb = api_mgr.begin_transaction();
    api_mgr.put(setup_txn_rb, key_rb_existing.clone(), value_rb_initial.clone());
    api_mgr.commit_transaction(setup_txn_rb);
    println!("主程序：设置 - 键 '{}' 设置为 '{}' 并已提交。", key_rb_existing, value_rb_initial);

    // 2. 开始一个新事务，在事务中修改现有键并添加新键
    let txn_id_rollback = api_mgr.begin_transaction();
    println!("主程序：已启动事务 {}", txn_id_rollback);

    api_mgr.put(txn_id_rollback, key_rb_existing.clone(), value_rb_txn_modified.clone());
    println!("主程序：事务 {} - 修改键 '{}' 为 '{}'", txn_id_rollback, key_rb_existing, value_rb_txn_modified);
    api_mgr.put(txn_id_rollback, key_rb_new.clone(), value_rb_new.clone());
    println!("主程序：事务 {} - 添加新键 '{}'，值为 '{}'", txn_id_rollback, key_rb_new, value_rb_new);

    // 3. 回滚事务
    api_mgr.rollback_transaction(txn_id_rollback);
    println!("主程序：事务 {} 已回滚。", txn_id_rollback);

    // 4. 验证数据
    // 验证现有键已回滚到初始值
    println!("\n主程序：尝试在回滚后获取键 '{}'：", key_rb_existing);
    match api_mgr.get(key_rb_existing.clone()) {
        Some(value) => {
            println!("主程序：回滚后获取 - 键 '{}' 的值：'{}'", key_rb_existing, value);
            assert_eq!(value, value_rb_initial, "键 key_rb_existing 的值应该已回滚到初始值");
        }
        None => println!("主程序：回滚后获取 - 未找到键 '{}' 的值。这是预料之外的。", key_rb_existing),
    }
    // 验证新键不存在
    println!("\n主程序：尝试在回滚后获取新键 '{}'：", key_rb_new);
    match api_mgr.get(key_rb_new.clone()) {
        Some(value) => println!("主程序：回滚后获取 - 新键 '{}' 的值：'{}'。这是预料之外的，因为它应该已被回滚。", key_rb_new, value),
        None => println!("主程序：回滚后获取 - 新键 '{}' 未找到，符合预期。", key_rb_new),
    }

    // 可选：演示 StorageEngine 的 get_version
    // 这需要直接访问 StorageEngine 或通过 ApiMgr 暴露一个接口
    // 为了简单起见，我们可以在这里假设如果需要，可以创建一个 StorageEngine 实例进行测试
    // 或者，如果 ApiMgr 内部的 TxnMgr 持有 StorageEngine，可以考虑添加一个调试接口
    println!("\n--- (可选) 演示版本控制 (如果可访问) ---");
    let key_version = "version_test_key".to_string();
    let val_v1 = "version1".to_string();
    let val_v2 = "version2".to_string();

    let txn_v1 = api_mgr.begin_transaction();
    api_mgr.put(txn_v1, key_version.clone(), val_v1.clone());
    api_mgr.commit_transaction(txn_v1);
    println!("主程序：已为键 '{}' 提交版本 1", key_version);

    let txn_v2 = api_mgr.begin_transaction();
    api_mgr.put(txn_v2, key_version.clone(), val_v2.clone());
    api_mgr.commit_transaction(txn_v2);
    println!("主程序：已为键 '{}' 提交版本 2", key_version);

    // 当前 ApiMgr 没有直接暴露 get_version。
    // 如果要测试，需要修改 ApiMgr 或直接使用 StorageEngine。
    // 此处仅为占位符，说明如何进行。
    // 例如: let version_0 = api_mgr.get_version(key_version.clone(), 0);
    // println!("主程序：键 '{}' 的版本 0：{:?}", key_version, version_0);
    // 实际的 StorageEngine::get_version(key, 0) 会返回第一个版本 (val_v1)

    println!("\n事务回滚和提交演示完成。");
}
