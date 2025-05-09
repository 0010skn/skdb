use std::fs;
use std::collections::HashMap;
use clap::Parser;
use skdb::{parse_dsl_input, execute_query, execute_update, execute_add, DslStatement, DslRoot, Table, Row, HeaderField, Value, TableData};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input .hs file to process
    #[clap(short, long, value_parser)]
    file: String,

    /// DSL statements to execute (queries or updates)
    #[clap(value_parser, required = true, num_args = 1..)]
    statements: Vec<String>,
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    let file_content = match fs::read_to_string(&args.file) {
        Ok(content) => content,
        Err(e) => {
            return Err(format!("Error reading file '{}': {}", args.file, e));
        }
    };

    let mut data_root: DslRoot = HashMap::new();
    
    println!("--- Parsing input file: {} ---", args.file);
    match parse_dsl_input(&file_content, None) { 
        Ok(parsed_file_statements) => {
            let mut definitions = Vec::new();
            let mut copy_ops = Vec::new();
            let mut ref_ops = Vec::new();
            let mut file_operations = Vec::new();

            for stmt in parsed_file_statements { 
                match stmt {
                    DslStatement::Definition(_, _) => definitions.push(stmt),
                    DslStatement::CopyStructure { .. } => copy_ops.push(stmt),
                    DslStatement::Reference { .. } => ref_ops.push(stmt),
                    DslStatement::Update { .. } | DslStatement::Add { .. } => file_operations.push(stmt),
                    DslStatement::Pack { .. } => eprintln!("Warning: 'pack' command in input file ignored by sk-runtime."),
                }
            }
            
            for stmt in copy_ops { 
                if let DslStatement::CopyStructure { source_table_name, source_path, target_table_name } = stmt {
                    println!("CLI: Processing #复制结构: source_table='{}', source_path='{}', target_table='{}'", source_table_name, source_path, target_table_name);
                     match fs::read_to_string(&source_path) {
                        Ok(src_file_content) => {
                            match parse_dsl_input(&src_file_content, None) {
                                Ok(source_statements) => {
                                    let mut found = false;
                                    for s_stmt in source_statements {
                                        if let DslStatement::Definition(name, table_def) = s_stmt {
                                            if name == source_table_name {
                                                let new_table = skdb::Table {
                                                    name: target_table_name.clone(),
                                                    headers: table_def.headers.clone(),
                                                    header_map: table_def.header_map.clone(),
                                                    data: skdb::TableData::Sequential(Vec::new()), 
                                                    primary_key_field_name: table_def.primary_key_field_name.clone(),
                                                };
                                                data_root.insert(target_table_name.clone(), new_table);
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                    if !found { eprintln!("CLI Error: Source table '{}' not found in '{}' for #复制结构.", source_table_name, source_path); }
                                }
                                Err(e) => {eprintln!("CLI Error: Parsing source file '{}' for #复制结构: {}", source_path, e);}
                            }
                        }
                        Err(e) => {eprintln!("CLI Error: Reading source file '{}' for #复制结构: {}", source_path, e);}
                    }
                }
            }

            for stmt in ref_ops { 
                if let DslStatement::Reference { source_table_name, source_path, target_table_name } = stmt {
                    println!("CLI: Processing #引用: source_table='{}', source_path='{}', target_table='{}'", source_table_name, source_path, target_table_name);
                    match fs::read_to_string(&source_path) {
                        Ok(src_file_content) => {
                            match parse_dsl_input(&src_file_content, None) {
                                Ok(source_statements) => {
                                    let mut found = false;
                                    for s_stmt in source_statements {
                                        if let DslStatement::Definition(name, mut table_def) = s_stmt {
                                            if name == source_table_name {
                                                if target_table_name != name { 
                                                    table_def.name = target_table_name.clone(); 
                                                }
                                                data_root.insert(target_table_name.clone(), table_def);
                                                found = true;
                                                break;
                                            }
                                        }
                                    }
                                     if !found { eprintln!("CLI Error: Source table '{}' not found in '{}' for #引用.", source_table_name, source_path); }
                                }
                                Err(e) => {eprintln!("CLI Error: Parsing source file '{}' for #引用: {}", source_path, e);}
                            }
                        }
                        Err(e) => {eprintln!("CLI Error: Reading source file '{}' for #引用: {}", source_path, e);}
                    }
                }
            }
            
            for stmt in definitions {
                if let DslStatement::Definition(name, parsed_block_table) = stmt {
                    if let Some(existing_table) = data_root.get_mut(&name) {
                        let is_from_copy_structure_init = matches!(existing_table.data, skdb::TableData::Sequential(ref v) if v.is_empty()) && existing_table.headers.is_empty(); 
                        
                        if is_from_copy_structure_init {
                            println!("CLI: Table '{}' (from #复制结构) needs data. Processing definition block.", name);
                            if let skdb::TableData::RawLines(raw_lines) = &parsed_block_table.data { 
                                if !existing_table.headers.is_empty() && !raw_lines.is_empty() { 
                                    let mut parsed_rows: Vec<skdb::Row> = Vec::new();
                                     for line_str in raw_lines {
                                        if line_str.trim().is_empty() || line_str.trim().starts_with('#') { continue; }
                                        match skdb::parser::parse_data_line(&line_str, &existing_table.headers, &existing_table.header_map) {
                                            Ok(row) => parsed_rows.push(row),
                                            Err(e) => eprintln!("CLI Error: Parsing data line '{}' for '{}': {}", line_str, name, e),
                                        }
                                    }
                                    existing_table.data = skdb::TableData::Sequential(parsed_rows);
                                    println!("CLI: Populated data for '{}' from RawLines.", name);
                                } else if existing_table.data.is_empty() { 
                                    if !parsed_block_table.headers.is_empty() && !parsed_block_table.data.is_empty() {
                                        existing_table.headers = parsed_block_table.headers;
                                        existing_table.header_map = parsed_block_table.header_map;
                                        existing_table.data = parsed_block_table.data;
                                        existing_table.primary_key_field_name = parsed_block_table.primary_key_field_name;
                                        println!("CLI: Table '{}' (from #复制结构) re-defined with new structure and data.", name);
                                    } else {
                                         println!("CLI: Table '{}' (from #复制结构) definition block has no RawLines data or target headers missing. Table remains empty.", name);
                                    }
                                }
                            } else if !parsed_block_table.data.is_empty() { 
                                 if existing_table.data.is_empty() || !parsed_block_table.headers.is_empty() { 
                                     existing_table.headers = parsed_block_table.headers; 
                                     existing_table.header_map = parsed_block_table.header_map;
                                     existing_table.data = parsed_block_table.data;
                                     existing_table.primary_key_field_name = parsed_block_table.primary_key_field_name;
                                     println!("CLI: Populated/Replaced data and structure for '{}' (from #复制结构) from fully parsed block.", name);
                                 }
                            } else {
                                 println!("CLI: Table '{}' (from #复制结构) definition block has no data. Table remains empty.", name);
                            }
                        } else {
                             println!("CLI Warning: Table '{}' (not an empty shell from #复制结构) encountered a subsequent definition block.", name);
                             println!("CLI: Replacing table '{}' with new definition.", name);
                             data_root.insert(name.clone(), parsed_block_table);
                        }
                    } else { 
                        data_root.insert(name.clone(), parsed_block_table);
                    }
                }
            }
            
            for stmt in file_operations { 
                 match stmt {
                    DslStatement::Update { path, value_str } => {
                        println!("CLI: File Update: {} = {}", path, value_str);
                        if let Err(e) = execute_update(&mut data_root, &path, &value_str) {
                             eprintln!("CLI Error: File update failed for '{}': {}", path, e);
                        }
                    }
                    DslStatement::Add { table_name } => {
                         println!("CLI: File Add to table: {}", table_name);
                        if let Err(e) = execute_add(&mut data_root, &table_name) {
                            eprintln!("CLI Error: File add failed for table '{}': {}", table_name, e);
                        }
                    }
                    _ => {} 
                }
            }
        }
        Err(e) => {
            return Err(format!("Error parsing input file '{}': {}", args.file, e));
        }
    }

    println!("--- Executing Command Line Statements ---");
    for stmt_str in args.statements {
        println!("Executing: {}", stmt_str);
        if stmt_str.contains('=') && stmt_str.starts_with("#.") { 
            let parts: Vec<&str> = stmt_str.splitn(2, '=').map(|s| s.trim()).collect();
            if parts.len() == 2 {
                let path = parts[0].strip_prefix("#.").unwrap_or(parts[0]); 
                let value_str = parts[1];
                match execute_update(&mut data_root, path, value_str) {
                    Ok(_) => println!("Update successful."),
                    Err(e) => eprintln!("Update failed: {}", e),
                }
            } else {
                eprintln!("Invalid update statement format: {}", stmt_str);
            }
        } else if stmt_str.starts_with('.') && stmt_str.contains(".add()") { 
             let table_name_part = stmt_str.trim_end_matches(".add()").strip_prefix('.').unwrap_or("");
             if !table_name_part.is_empty() {
                match execute_add(&mut data_root, table_name_part) {
                    Ok(_) => println!("Add to table '{}' successful.", table_name_part),
                    Err(e) => eprintln!("Add to table '{}' failed: {}", table_name_part, e),
                }
             } else {
                eprintln!("Invalid add statement format: {}", stmt_str);
             }
        }
        else if stmt_str.starts_with("#.") { 
            match execute_query(&data_root, &stmt_str) {
                Some(value) => println!("Query result: {:?}", value),
                None => println!("Query result: Not found or error in path."),
            }
        } else {
            eprintln!("Unsupported statement format: {}. Must start with '#.' for query/update or '.table.add()' for add.", stmt_str);
        }
    }

    println!("--- Persisting changes to {} ---", args.file);
    let mut output_content = String::new();

    fn format_value(value: &Value) -> String {
        match value {
            Value::String(s) => {
                // Consistent with how parser.rs's parse_value_str expects quoted strings
                // if they contain special characters or are ambiguous.
                if s.contains(',') || s.contains('(') || s.contains(')') || s.contains('"') || s.starts_with(' ') || s.ends_with(' ') || s.is_empty() {
                    format!("\"{}\"", s.replace("\"", "\\\"")) // Escape inner quotes
                } else {
                    s.clone()
                }
            }
            Value::Integer(i) => i.to_string(),
            Value::Tuple(values) => {
                let inner: Vec<String> = values.iter().map(|v| format_value(v)).collect(); // Recursive call
                format!("({})", inner.join(","))
            }
            Value::Reference { type_name, key } => {
                // Recursively format the key part to ensure it's also correctly stringified
                let key_str = format_value(key.as_ref());
                format!("{}::{}", type_name, key_str)
            }
            Value::Null => "null".to_string(), // Output "null" for Value::Null, parser handles this.
        }
    }

    let format_row_custom = |row: &Row, headers: &Vec<HeaderField>| -> String {
        let mut line_parts = Vec::new();
        for header_field in headers {
            if let Some(value) = row.fields.get(&header_field.name) {
                line_parts.push(format_value(value));
            } else {
                line_parts.push("".to_string());
            }
        }
        line_parts.join(",") 
    };

    for table in data_root.values() {
        output_content.push_str(&format!("{}:\n", table.name)); 
        
        if !table.headers.is_empty() {
            let header_string = table.headers.iter().map(|h| {
                if let Some(type_info) = &h.type_info {
                    format!("{}::{}", h.name, type_info)
                } else {
                    h.name.clone()
                }
            }).collect::<Vec<String>>().join("/");
            output_content.push_str(&format!("/{}/\n", header_string)); 
        }

        match &table.data {
            TableData::Sequential(rows) => {
                for row in rows {
                    output_content.push_str(&format!("{}\n", format_row_custom(row, &table.headers)));
                }
            }
            TableData::Indexed(map) => {
                let mut sorted_keys: Vec<&String> = map.keys().collect();
                sorted_keys.sort(); 
                for key in sorted_keys {
                    if let Some(row) = map.get(key) {
                        output_content.push_str(&format!("{}\n", format_row_custom(row, &table.headers)));
                    }
                }
            }
            TableData::GroupedIndexed(map) => {
                let mut sorted_group_keys: Vec<&String> = map.keys().collect();
                sorted_group_keys.sort();
                for group_key in sorted_group_keys {
                    if let Some(rows_in_group) = map.get(group_key) {
                        for row in rows_in_group {
                            output_content.push_str(&format!("{}\n", format_row_custom(row, &table.headers)));
                        }
                    }
                }
            }
            TableData::RawLines(lines) => {
                eprintln!("[WARN] Table '{}' is being written from RawLines. Data might not conform to expected format.", table.name);
                for line in lines { 
                    output_content.push_str(&format!("{}\n", line));
                }
            }
        }
        output_content.push_str("~\n"); 
    }
    
    let final_content = if output_content.is_empty() {
        "\n".to_string() 
    } else {
        let mut temp_content = output_content;
        if !temp_content.ends_with('\n') { 
            temp_content.push('\n');
        }
        let mut trimmed_content = temp_content.trim_end_matches(|c: char| c == '\n' || c == '\r').to_string();
        trimmed_content.push('\n');
        trimmed_content
    };

    match fs::write(&args.file, final_content) {
        Ok(_) => println!("Successfully wrote changes to {}", args.file),
        Err(e) => {
            return Err(format!("Error writing changes to file '{}': {}", args.file, e));
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}