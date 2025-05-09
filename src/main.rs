// Assuming the library crate is named 'skdb' as per the project structure.
// This name should match the one in Cargo.toml: [package] name = "skdb"
// or if it's a library, it's usually the directory name.
// If src/lib.rs exists, Cargo treats the package as a library,
// and src/main.rs becomes a binary that can use this library.
use skdb::{parse_dsl_input, execute_query, execute_update, execute_add, execute_pack, DslStatement, DslRoot}; // Removed Value
use std::collections::HashMap;
use std::fs; // Import the fs module

fn main() {
    // mock_fs is no longer used, direct file I/O will be used.

    let dsl_definitions = r#"
#复制结构 original_table from external.hs as cloned_table
#引用 other_original_table from external.hs
#复制结构 regions from hs/extra_data1.hs as regions_imported 
#复制结构 products from hs/extra_data2.hs as products_imported
~
user:
/id::sindex/name/c::config/
0,user1,(100,1999999,call1)
1,user2,(200,,call2)
~
config:
/id:index/gold/time/system_ref::system/
key1,1000,2024,sys_A
key2,2000,2025,sys_B
(100,1999999,call1),10,20,(call1_sys)
~
system:
/id:index/call/
sys_A,SystemCallA
sys_B,SystemCallB
(call1_sys),ActualCall1Value
~
project_config:
/name/system_ref::system/
项目名字,(callxx_sys)
~
assembly:
/id:gindex/p_ref::project_config/
ios,(项目1_config_key,)
ios,(项目2_config_key,)
android,(项目1_config_key,)
android,(项目2_config_key,_)
~
names:
/id:index/c_ref::content/
china,('hello,china_content_key')
~
content:
/id:index/content_val/
'hello,china_content_key',实际内容是你好中国
~
minimal_user:
/id::sindex/data/
0,userdata0
~
minimal_config:
/key:index/value/
cfg1,config_value_1
~
user_ref_config:
/id::sindex/conf_key::minimal_config/
0,cfg1
~
project_config_indexed:
/name:index/system_ref::system/
项目1_config_key,(call_proj1_sys)
项目2_config_key,(call_proj2_sys)
~
cloned_table: // Data for the cloned table, structure comes from external.hs
0,data_for_cloned_0
1,data_for_cloned_1
~
other_original_table: // Data for the referenced table
key0,new_data_for_key0
key1,data_for_key1
~
another_table:
/id::sindex/ref_to_cloned::cloned_table/ref_to_other::other_original_table/
0,(0),(key0)
1,(1),(key1)
~
regions_imported:
AS,Asia,4700000000
EU,Europe,750000000
AF,Africa,1300000000
~
products_imported:
P101,Laptop,Electronics
P102,Desk Chair,Furniture
~
"#.to_string();

    // Separate operations for clarity and to ensure definitions are processed first
    let dsl_operations = r#"
#.user[0].name = '张三丰'
#.config[key1].gold = 9999
#.minimal_user[0].data = '更新后的用户数据'
.user.add()
#.user[2].name = '新添加用户'
#.user[2].c = _
.user.add()
#.user[3].name = '再一个用户'
#.user[3].id = 3 // This should be auto-assigned by add_row for sindex, but testing if update works

# Pack some tables
pack user config project_config assembly cloned_table other_original_table another_table names content minimal_user minimal_config user_ref_config project_config_indexed regions_imported products_imported
"#.to_string();

    let full_dsl_input = format!("{}{}", dsl_definitions, dsl_operations);

    println!("--- Parsing DSL Input ---");
    match parse_dsl_input(&full_dsl_input, None) {
        Ok(parsed_statements) => {
            let mut data_root: DslRoot = HashMap::new();
            let mut definitions = Vec::new();
            let mut operations = Vec::new();
            let mut copy_operations = Vec::new();
            let mut reference_operations = Vec::new(); 
            let mut pack_operations = Vec::new();

            // Separate statements for phased processing
            for stmt in parsed_statements {
                match stmt {
                    DslStatement::Definition(_, _) => definitions.push(stmt),
                    DslStatement::Update { .. } | DslStatement::Add { .. } => operations.push(stmt),
                    DslStatement::CopyStructure { .. } => copy_operations.push(stmt),
                    DslStatement::Reference { .. } => reference_operations.push(stmt),
                    DslStatement::Pack { .. } => pack_operations.push(stmt),
                }
            }

            println!("--- Phase 1: Processing CopyStructure and Reference Directives ---");
            for stmt in copy_operations {
                if let DslStatement::CopyStructure { source_table_name, source_path, target_table_name } = stmt {
                    println!("Processing #复制结构: source_table='{}', source_path='{}', target_table='{}'", source_table_name, source_path, target_table_name);
                    match fs::read_to_string(&source_path) {
                        Ok(file_content) => {
                            match parse_dsl_input(&file_content, None) {
                                Ok(source_statements) => {
                                    let mut found_source_table = false;
                                    for source_stmt in source_statements {
                                        if let DslStatement::Definition(name, table_def) = source_stmt {
                                            if name == source_table_name {
                                                println!("Found source table '{}' in '{}'. Cloning structure to '{}'.", source_table_name, source_path, target_table_name); 
                                                let new_table = skdb::Table {
                                                    name: target_table_name.clone(),
                                                    headers: table_def.headers.clone(), 
                                                    header_map: table_def.header_map.clone(),
                                                    data: skdb::TableData::Sequential(Vec::new()), 
                                                    primary_key_field_name: table_def.primary_key_field_name.clone(),
                                                };
                                                data_root.insert(target_table_name.clone(), new_table);
                                                found_source_table = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found_source_table {
                                        eprintln!("Error: Source table '{}' not found in file '{}'.", source_table_name, source_path);
                                    }
                                }
                                Err(e) => {eprintln!("Error parsing source file '{}': {}", source_path, e);} 
                            } 
                        } 
                        Err(e) => {eprintln!("Error reading source file '{}': {}", source_path, e);} 
                    } 
                } 
            } 

            for stmt in reference_operations {
                if let DslStatement::Reference { source_table_name, source_path, target_table_name } = stmt {
                    println!("Processing #引用: source_table='{}', source_path='{}', target_table='{}'", source_table_name, source_path, target_table_name);
                    match fs::read_to_string(&source_path) {
                        Ok(file_content) => {
                            match parse_dsl_input(&file_content, None) {
                                Ok(source_statements) => {
                                    let mut found_and_referenced_table = false;
                                    for source_stmt in source_statements {
                                        if let DslStatement::Definition(name, mut table_def) = source_stmt {
                                            if name == source_table_name {
                                                println!("Found source table '{}' in '{}'. Referencing as '{}'.", source_table_name, source_path, target_table_name);
                                                if target_table_name != name {
                                                    table_def.name = target_table_name.clone();
                                                }
                                                data_root.insert(target_table_name.clone(), table_def); 
                                                found_and_referenced_table = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found_and_referenced_table {
                                        eprintln!("Error: Source table '{}' not found in file '{}' for #引用.", source_table_name, source_path);
                                    }
                                }
                                Err(e) => {eprintln!("Error parsing source file '{}' for #引用: {}", source_path, e);} 
                            } 
                        } 
                        Err(e) => {eprintln!("Error reading source file '{}' for #引用: {}", source_path, e);} 
                    } 
                } 
            } 

            println!("--- Phase 2: Processing Definitions ---");
            for stmt in definitions {
                if let DslStatement::Definition(name, parsed_block_table) = stmt {
                    if let Some(existing_table) = data_root.get_mut(&name) {
                        
                        let is_from_copy_structure_init = matches!(existing_table.data, skdb::TableData::Sequential(ref v) if v.is_empty());

                        if !is_from_copy_structure_init {
                           println!("Warning: Table '{}' (likely from #引用 or already has data) encountered a new definition block. This block will be ignored.", name);
                           continue; 
                        }

                        println!("Table '{}' (from #复制结构) needs data. Processing data from this block.", name);
                        match parsed_block_table.data {
                            skdb::TableData::RawLines(raw_lines) => {
                                if raw_lines.is_empty() {
                                    println!("Data block for '{}' is empty.", name);
                                    continue;
                                }
                                if existing_table.headers.is_empty() {
                                    eprintln!("Error: Cannot process raw data lines for table '{}' (from #复制结构) because its structure is missing.", name);
                                    continue;
                                }
                                println!("Parsing {} raw data lines for table '{}' using existing headers.", raw_lines.len(), name);
                                let mut parsed_rows: Vec<skdb::Row> = Vec::new();
                                for line_str in raw_lines {
                                    if line_str.trim().is_empty() || line_str.trim().starts_with('#') {
                                        continue;
                                    }
                                    match skdb::parser::parse_data_line(&line_str, &existing_table.headers, &existing_table.header_map) {
                                        Ok(row) => parsed_rows.push(row),
                                        Err(e) => eprintln!("Error parsing data line '{}' for table '{}': {}", line_str, name, e),
                                    }
                                }

                                if let Some(pk_name) = &existing_table.primary_key_field_name {
                                    let pk_header_field = existing_table.headers.iter().find(|h| &h.name == pk_name);
                                    let pk_type = pk_header_field.and_then(|h| h.type_info.as_deref());
                                    match pk_type {
                                        Some("index") => {
                                            let mut indexed_data = HashMap::new();
                                            for row in parsed_rows {
                                                if let Some(pk_value) = row.fields.get(pk_name) {
                                                    if let Ok(key_str) = skdb::parser::value_to_string_key(pk_value) {
                                                        indexed_data.insert(key_str, row);
                                                    } else { eprintln!("Error creating key for indexed table '{}'", name); }
                                                } else { eprintln!("PK '{}' not found in row for indexed table '{}'", pk_name, name); }
                                            }
                                            existing_table.data = skdb::TableData::Indexed(indexed_data);
                                        }
                                        Some("gindex") => {
                                            let mut grouped_data = HashMap::new();
                                            for row in parsed_rows {
                                                if let Some(pk_value) = row.fields.get(pk_name) {
                                                    if let Ok(key_str) = skdb::parser::value_to_string_key(pk_value) {
                                                        grouped_data.entry(key_str).or_insert_with(Vec::new).push(row);
                                                    } else { eprintln!("Error creating key for gindexed table '{}'", name); }
                                                } else { eprintln!("PK '{}' not found in row for gindexed table '{}'", pk_name, name); }
                                            }
                                            existing_table.data = skdb::TableData::GroupedIndexed(grouped_data);
                                        }
                                        _ => { existing_table.data = skdb::TableData::Sequential(parsed_rows); } 
                                    }
                                } else {
                                    existing_table.data = skdb::TableData::Sequential(parsed_rows);
                                }
                                println!("Successfully populated data for table '{}'.", name);
                            }
                            _ => { 
                                 if existing_table.data.is_empty() { 
                                     println!("Warning: Table '{}' (from #复制结构) is being populated by a Definition block that also parsed its own headers/data. Overwriting with new data.", name);
                                     existing_table.data = parsed_block_table.data;
                                 } else {
                                     println!("Warning: Table '{}' (from #复制结构) already has data or was from #引用, and Definition block also has data. New data block ignored.", name);
                                 }
                            }
                        }
                    } else {
                        println!("Defining new table: {}", name);
                        data_root.insert(name, parsed_block_table);
                    }
                }
            }
            
            println!("--- End of Phase 2 ---");
            if let Some(table) = data_root.get("content") {
                if let skdb::TableData::Indexed(map) = &table.data {
                    println!("DEBUG main.rs: 'content' table keys in data_root: {:?}", map.keys().collect::<Vec<_>>());
                } else {
                     println!("DEBUG main.rs: 'content' table data is not Indexed: {:?}", table.data);
                }
            } else {
                 println!("DEBUG main.rs: 'content' table not found in data_root");
            }
            if let Some(table) = data_root.get("config") {
                if let skdb::TableData::Indexed(map) = &table.data {
                    println!("DEBUG main.rs: 'config' table keys in data_root: {:?}", map.keys().collect::<Vec<_>>());
                } else {
                     println!("DEBUG main.rs: 'config' table data is not Indexed: {:?}", table.data);
                }
            } else {
                 println!("DEBUG main.rs: 'config' table not found in data_root");
            }

            println!("--- Phase 3: Processing Operations (Update, Add) ---");
            for stmt in operations {
                 match stmt {
                    DslStatement::Update { path, value_str } => {
                        println!("Executing update: {} = {}", path, value_str);
                        match execute_update(&mut data_root, &path, &value_str) { 
                            Ok(_) => println!("Update successful."),
                            Err(e) => eprintln!("Update failed for '{}': {}", path, e),
                        }
                    }
                    DslStatement::Add { table_name } => {
                        println!("Executing add to table: {}", table_name);
                        match execute_add(&mut data_root, &table_name) {
                            Ok(_) => println!("Add successful."),
                            Err(e) => eprintln!("Add failed for table '{}': {}", table_name, e),
                        }
                    }
                    _ => {} 
                }
            }

            println!("--- Phase 4: Processing Pack Operations ---");
            if !pack_operations.is_empty() {
                for stmt in pack_operations {
                    if let DslStatement::Pack { table_names } = stmt {
                        println!("Executing pack for tables: {:?}", table_names);
                        match execute_pack(&data_root, &table_names) {
                            Ok(packed_string) => {
                                println!("--- Packed Output ---");
                                println!("{}", packed_string);
                                println!("--- End of Packed Output ---");

                                let output_file_path = "packed_output.hs";
                                match fs::write(output_file_path, &packed_string) {
                                    Ok(_) => println!("Packed output successfully written to {}", output_file_path),
                                    Err(e) => eprintln!("Error writing packed output to {}: {}", output_file_path, e),
                                }

                                println!("\n--- Verifying Packed Output (Round-trip Test) ---");
                                match parse_dsl_input(&packed_string, None) {
                                    Ok(re_parsed_statements) => {
                                        let mut round_trip_root: DslRoot = HashMap::new();
                                        for stmt_rt in re_parsed_statements { 
                                            if let DslStatement::Definition(name, table) = stmt_rt {
                                                round_trip_root.insert(name, table);
                                            } else {
                                                eprintln!("Warning (Round-trip): Non-definition statement: {:?}", stmt_rt);
                                            }
                                        }
                                        println!("Re-parsed {} tables from packed output.", round_trip_root.len());

                                        if !table_names.is_empty() {
                                            let first_table_to_check = &table_names[0];
                                            if let Some(table_detail) = round_trip_root.get(first_table_to_check) {
                                                if !table_detail.headers.is_empty() {
                                                    let first_field_to_check = &table_detail.headers[0].name;
                                                    let query_str_rt = match table_detail.get_index_type() {
                                                        Some("sindex") => {
                                                            format!("#.{}[0].{}", first_table_to_check, first_field_to_check)
                                                        }
                                                        Some("index") | Some("gindex") => {
                                                            let mut key_to_query: Option<String> = None;
                                                            if let Some(original_table_for_key) = data_root.get(first_table_to_check) {
                                                                match &original_table_for_key.data {
                                                                    skdb::TableData::Indexed(map) => {
                                                                        if let Some(first_key) = map.keys().next() {
                                                                            key_to_query = Some(format!("{}", first_key));
                                                                        }
                                                                    }
                                                                    skdb::TableData::GroupedIndexed(map) => {
                                                                         if let Some(first_key) = map.keys().next() {
                                                                            key_to_query = Some(format!("{}", first_key));
                                                                        }
                                                                    }
                                                                    _ => {}
                                                                }
                                                            }
                                                            if let Some(k) = key_to_query {
                                                                if table_detail.get_index_type() == Some("gindex") {
                                                                    println!("Note (Round-trip): For gindexed table '{}', using key '{}'. Querying first field of first row in group.", first_table_to_check, k);
                                                                    format!("#.{}[{}][0].{}", first_table_to_check, k, first_field_to_check)
                                                                } else {
                                                                    format!("#.{}[{}].{}", first_table_to_check, k, first_field_to_check)
                                                                }
                                                            } else {
                                                                println!("Skipping round-trip query for indexed table '{}' (no key found).", first_table_to_check);
                                                                "".to_string()
                                                            }
                                                        }
                                                        _ => { 
                                                            if table_detail.data.get_sequential_row(0).is_some() {
                                                                 format!("#.{}[0].{}", first_table_to_check, first_field_to_check)
                                                            } else {
                                                                println!("Skipping round-trip query for table '{}' (no rows or not sindex).", first_table_to_check);
                                                                "".to_string()
                                                            }
                                                        }
                                                    };

                                                    if !query_str_rt.is_empty() {
                                                        println!("Attempting round-trip query: {}", query_str_rt);
                                                        match execute_query(&round_trip_root, &query_str_rt) {
                                                            Some(val) => println!("Round-trip query for '{}' -> Some({:?})", query_str_rt, val),
                                                            None => println!("Round-trip query for '{}' -> None", query_str_rt),
                                                        }
                                                    }
                                                } else {
                                                    println!("Round-trip: Table '{}' has no headers.", first_table_to_check);
                                                }
                                            } else {
                                                 println!("Round-trip: Table '{}' not found.", first_table_to_check);
                                            }
                                        }
                                    } 
                                    Err(e) => { 
                                        eprintln!("Failed to re-parse packed output: {}", e);
                                    }
                                } 
                            } 
                            Err(e) => { 
                                eprintln!("Pack operation failed: {}", e);
                            }
                        } 
                    } 
                } 
            } else {
                println!("No pack operations to process.");
            }
            
            println!("\n--- Testing Queries After Operations ---");
            
            let queries_to_test: Vec<String> = match fs::read_to_string("queries/test_queries.hsg") {
                Ok(content) => {
                    content.lines()
                        .map(|line| line.trim())
                        .filter(|line| !line.is_empty() && !line.starts_with("//")) 
                        .filter(|line| line.starts_with("#.")) 
                        .map(|line| line.to_string())
                        .collect()
                }
                Err(e) => {
                    eprintln!("Error reading query file 'queries/test_queries.hsg': {}", e);
                    Vec::new() 
                }
            };

            if queries_to_test.is_empty() {
                 println!("No queries found in 'queries/test_queries.hsg' or file could not be read.");
            } else {
                println!("Loaded {} queries from 'queries/test_queries.hsg'", queries_to_test.len());
            }

            for q_str in &queries_to_test { 
                match execute_query(&data_root, q_str) {
                    Some(value) => println!("Query '{}': {:?}", q_str, value),
                    None => println!("Query '{}': Not found or error in path", q_str),
                }
            }

            println!("\n--- Querying newly added user's potentially null field ---");
            match execute_query(&data_root, "#.user[2].c") {
                 Some(value) => println!("Query '#.user[2].c': {:?}", value), 
                 None => println!("Query '#.user[2].c': Not found or error in path"),
            }
        }
        Err(e) => {eprintln!("Error parsing DSL input: {}", e);}
    }
}
