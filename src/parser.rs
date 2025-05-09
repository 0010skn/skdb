use std::collections::HashMap;
use crate::structs::{Value, HeaderField, Row, TableData, Table}; 

#[derive(Debug, PartialEq)]
pub enum DslStatement {
    Definition(String, Table), 
    Update { path: String, value_str: String }, 
    Add { table_name: String }, 
    CopyStructure {
        source_table_name: String,
        source_path: String, 
        target_table_name: String,
    },
    Reference { 
        source_table_name: String,
        source_path: String,
        target_table_name: String,
    },
    Pack { table_names: Vec<String> },
}

// Helper to parse arguments for pack, expecting "table1 table2 ..."
fn parse_pack_args(args_line: &str) -> Result<DslStatement, String> {
    let command_content = args_line.split('#').next().unwrap_or("").trim(); 

    if command_content.is_empty() {
        return Err("Pack arguments must specify at least one table name.".to_string());
    }

    let table_names: Vec<String> = command_content
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if table_names.is_empty() || table_names.iter().any(|name| name.is_empty()) {
        return Err("Table names in pack statement cannot be empty or missing.".to_string());
    }

    Ok(DslStatement::Pack { table_names })
}


pub fn parse_dsl_input(input: &str, _mock_fs: Option<&HashMap<String, String>>) -> Result<Vec<DslStatement>, String> {
    let mut statements = Vec::new();
    let mut current_block_lines: Vec<String> = Vec::new();
    let mut in_block = false;

    for line_with_ending in input.lines() {
        let line = line_with_ending.trim_end_matches(['\r', '\n']); 
        let trimmed_line = line.trim(); 

        if trimmed_line.is_empty() {
            continue;
        }
        
        if trimmed_line.starts_with("##") { // Double hash for full line comment
            continue;
        }
        if trimmed_line.starts_with('#') { // Any other line starting with # is a comment
            continue; 
        }

        let mut processed_as_statement = false;

        // 1. Check for known keyword-prefixed directives/operations first
        if trimmed_line.starts_with("pack ") {
            if let Some(args_part) = trimmed_line.strip_prefix("pack ").map(|s| s.trim_start()) {
                if in_block && !current_block_lines.is_empty() {
                    let block_str = current_block_lines.join("\n");
                    if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
                    current_block_lines.clear(); in_block = false;
                }
                statements.push(parse_pack_args(args_part)?);
                processed_as_statement = true;
            }
        } else if trimmed_line.starts_with("#.") { 
            if in_block && !current_block_lines.is_empty() {
                let block_str = current_block_lines.join("\n");
                if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
                current_block_lines.clear(); in_block = false;
            }
            statements.push(parse_update_statement(trimmed_line)?);
            processed_as_statement = true;
        } else if trimmed_line.starts_with('.') && !trimmed_line.starts_with("..") { 
             if in_block && !current_block_lines.is_empty() {
                let block_str = current_block_lines.join("\n");
                 if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
                current_block_lines.clear(); in_block = false;
            }
            statements.push(parse_add_statement(trimmed_line)?);
            processed_as_statement = true;
        }

        if processed_as_statement {
            continue;
        }

        // 2. Try to parse as "keyword-less" (pattern-based) directives if not a table definition start
        // A table definition must start with "name:", so if a line doesn't contain ":" it might be a keyword-less directive.
        // Or, more robustly, check for " from " and " as " patterns.
        let parts: Vec<&str> = trimmed_line.split_whitespace().collect();
        let mut parsed_as_keywordless_directive = false;

        if !trimmed_line.contains(':') { // Heuristic: table names usually end with ':' on their line
            if parts.len() == 5 && parts[1] == "from" && parts[3] == "as" {
                // Potential CopyStructure: source_table from "path" as target_table
                let source_table_name = parts[0].to_string();
                let source_path = parts[2].trim_matches('"').to_string();
                let target_table_name = parts[4].to_string();
                if !source_table_name.is_empty() && !source_path.is_empty() && !target_table_name.is_empty() && !source_table_name.contains(':') {
                    if in_block && !current_block_lines.is_empty() {
                        let block_str = current_block_lines.join("\n");
                        if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
                        current_block_lines.clear(); in_block = false;
                    }
                    statements.push(DslStatement::CopyStructure{ source_table_name, source_path, target_table_name });
                    parsed_as_keywordless_directive = true;
                }
            } else if parts.len() == 3 && parts[1] == "from" {
                // Potential Reference: source_table from "path"
                let source_table_name = parts[0].to_string();
                let source_path = parts[2].trim_matches('"').to_string();
                if !source_table_name.is_empty() && !source_path.is_empty() && !source_table_name.contains(':') {
                     if in_block && !current_block_lines.is_empty() {
                        let block_str = current_block_lines.join("\n");
                        if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
                        current_block_lines.clear(); in_block = false;
                    }
                    statements.push(DslStatement::Reference{ source_table_name: source_table_name.clone(), source_path, target_table_name: source_table_name });
                    parsed_as_keywordless_directive = true;
                }
            }
        }

        if parsed_as_keywordless_directive {
            continue;
        }

        // 3. If none of the above, assume it's part of a table definition block
        current_block_lines.push(line.to_string()); 
        in_block = true;

        if line.contains('~') {
            let parts_before_tilde: Vec<&str> = line.splitn(2, '~').collect();
            let content_for_current_block = parts_before_tilde[0].trim_end(); 
            
            if !current_block_lines.is_empty() { current_block_lines.pop(); }
            if !content_for_current_block.is_empty() { current_block_lines.push(content_for_current_block.to_string()); }

            if !current_block_lines.is_empty() {
                let block_str = current_block_lines.join("\n");
                 if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
            }
            current_block_lines.clear(); in_block = false;

            if parts_before_tilde.len() > 1 {
                let content_after_tilde = parts_before_tilde[1].trim();
                if !content_after_tilde.is_empty() {
                    // This could be the start of a new block or a new keyword-less directive.
                    // Re-feed to a simplified parsing logic or assume it's a new block start.
                    // For simplicity, assume it's a new block start if it contains ':'.
                    // This part is tricky and might need a lookahead or a more formal grammar.
                    // For now, if it's not empty, push it to start a new block.
                    current_block_lines.push(content_after_tilde.to_string());
                    in_block = true; 
                }
            }
        }
    }

    if in_block && !current_block_lines.is_empty() {
        let block_str = current_block_lines.join("\n");
        if !block_str.trim().is_empty() { statements.push(DslStatement::Definition(parse_block(&block_str)?.0, parse_block(&block_str)?.1));}
    }

    Ok(statements)
}


fn parse_update_statement(line: &str) -> Result<DslStatement, String> {
    let parts: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
    if parts.len() != 2 {
        return Err(format!("Invalid update statement format: '{}'. Expected '#.path = value'", line));
    }
    let path = parts[0].strip_prefix("#.").ok_or_else(|| format!("Update path missing '#.' prefix: '{}'", parts[0]))?.to_string();
    if path.is_empty() {
        return Err(format!("Update path cannot be empty in: '{}'", line));
    }
    let value_str = parts[1].to_string();

    Ok(DslStatement::Update { path, value_str })
}

fn parse_add_statement(line: &str) -> Result<DslStatement, String> {
    let trimmed_line = line.trim();
    if !trimmed_line.ends_with(".add()") {
        return Err(format!("Invalid add statement format: '{}'. Expected '.table_name.add()'", line));
    }
    
    let table_name_part = trimmed_line.trim_end_matches(".add()").strip_prefix('.')
        .ok_or_else(|| format!("Invalid table name for add operation: '{}'", line))?;
    
    if table_name_part.is_empty() || table_name_part.contains('.') || table_name_part.contains('[') || table_name_part.contains(']') {
        return Err(format!("Invalid table name for add operation: '{}'. Must be a simple name.", table_name_part));
    }

    Ok(DslStatement::Add { table_name: table_name_part.to_string() })
}


fn parse_block(block_str: &str) -> Result<(String, Table), String> {
    let lines: Vec<&str> = block_str.lines()
                                    .map(|s| s.trim())
                                    .filter(|s| !s.is_empty() && !s.starts_with("##")) 
                                    .collect();
    if lines.is_empty() {
        return Err("Block is empty or only contains comments".to_string());
    }

    let name_line_full = lines[0];
    if name_line_full.trim_start().starts_with('#') { // A line starting with # should have been skipped by parse_dsl_input
        return Err(format!("Table definition block cannot start with a comment line that was not filtered: '{}'", name_line_full));
    }
    let name_line_no_comment = name_line_full.split('#').next().unwrap_or("").split("//").next().unwrap_or("").trim();
    let table_name = name_line_no_comment.strip_suffix(':').ok_or_else(|| format!("Invalid table name format: '{}'. Expected 'name:' for line '{}'", name_line_no_comment, name_line_full))?.to_string();
    
    let mut is_header_line_present = false;
    if lines.len() >= 2 {
        let potential_header_line_full = lines[1];
        let line_for_header_check = if potential_header_line_full.trim_start().starts_with('#') {
            "" 
        } else {
            potential_header_line_full
        };
        let potential_header_line_no_comment = line_for_header_check.split('#').next().unwrap_or("").split("//").next().unwrap_or("").trim();
        if potential_header_line_no_comment.starts_with('/') && potential_header_line_no_comment.ends_with('/') {
            is_header_line_present = true;
        }
    }

    if !is_header_line_present {
        let raw_data_lines: Vec<String> = if lines.len() > 1 {
            lines.iter().skip(1)
                 .map(|s| s.split('#').next().unwrap_or("").split("//").next().unwrap_or("").trim()) 
                 .filter(|s| !s.is_empty()) 
                 .map(|s| s.to_string())
                 .collect()
        } else {
            Vec::new() 
        };

        return Ok((
            table_name.clone(),
            Table {
                name: table_name,
                headers: Vec::new(), 
                header_map: HashMap::new(),
                data: TableData::RawLines(raw_data_lines),
                primary_key_field_name: None, 
            },
        ));
    }

    let header_line_full = lines[1];
    let header_str_no_comment = header_line_full.split('#').next().unwrap_or("").split("//").next().unwrap_or("").trim();
    let (headers, primary_key_field_name_from_header) = parse_header_line(header_str_no_comment)?;
    
    let mut header_map = HashMap::new();
    for (i, h) in headers.iter().enumerate() {
        header_map.insert(h.name.clone(), i);
    }

    let mut data_rows: Vec<Row> = Vec::new();
    for data_line_full in lines.iter().skip(2) {
        let data_line_content = data_line_full.split('#').next().unwrap_or("").split("//").next().unwrap_or("").trim();
        
        if data_line_content.is_empty() { 
            continue;
        }
        data_rows.push(parse_data_line(data_line_content, &headers, &header_map)?);
    }
    
    let table_data;
    let final_primary_key_field_name = primary_key_field_name_from_header.clone();

    if let Some(pk_name) = &final_primary_key_field_name {
        let pk_header_opt = headers.iter().find(|h| h.name == *pk_name && h.is_primary_key);
        if pk_header_opt.is_none() {
            return Err(format!("Primary key field '{}' (from id:type notation) not found or not marked as PK in header fields for table '{}'", pk_name, table_name));
        }

        let pk_type_info = pk_header_opt.unwrap().type_info.as_deref();
        match pk_type_info {
            Some("index") => {
                let mut indexed_data = HashMap::new();
                for row in data_rows {
                    if let Some(pk_value) = row.fields.get(pk_name) {
                        indexed_data.insert(value_to_string_key(pk_value)?, row);
                    } else {
                        return Err(format!("Primary key field '{}' not found in a data row for indexed table '{}'", pk_name, table_name));
                    }
                }
                table_data = TableData::Indexed(indexed_data);
            }
            Some("gindex") => {
                let mut grouped_data = HashMap::new();
                for row in data_rows {
                    if let Some(pk_value) = row.fields.get(pk_name) {
                        grouped_data.entry(value_to_string_key(pk_value)?).or_insert_with(Vec::new).push(row);
                    } else {
                         return Err(format!("Primary key field '{}' not found in a data row for gindexed table '{}'", pk_name, table_name));
                    }
                }
                table_data = TableData::GroupedIndexed(grouped_data);
            }
            Some("sindex") | _ => { 
                table_data = TableData::Sequential(data_rows);
            }
        }
    } else {
        if headers.iter().any(|h| h.type_info.as_deref() == Some("sindex")) {
            table_data = TableData::Sequential(data_rows);
        } else {
            table_data = TableData::Sequential(data_rows);
        }
    }

    Ok((table_name.clone(), Table {
        name: table_name,
        headers,
        header_map,
        data: table_data,
        primary_key_field_name: final_primary_key_field_name,
    }))
}

pub fn value_to_string_key(value: &Value) -> Result<String, String> { 
    match value {
        Value::String(s) => Ok(s.clone()),
        Value::Integer(i) => Ok(i.to_string()),
        Value::Tuple(vals) => {
            let inner_keys: Result<Vec<String>, String> = vals.iter().map(|v| {
                 value_to_string_key(v)
            }).collect();

            inner_keys.map(|keys| format!("({})", keys.join(","))) 
        }
        Value::Reference { .. } => Err("Reference cannot be used as a direct table key.".to_string()), 
        Value::Null => Err("Null cannot be used as a table key.".to_string()),
    }
}

fn parse_header_line(line: &str) -> Result<(Vec<HeaderField>, Option<String>), String> {
    if !line.starts_with('/') || !line.ends_with('/') {
        return Err(format!("Header line must start and end with '/': {}", line));
    }
    let inner = &line[1..line.len() - 1];
    if inner.is_empty() && line == "/" { 
        return Ok((Vec::new(), None));
    }
    if inner.is_empty() && line == "//" { 
        return Ok((Vec::new(), None));
    }


    let mut headers = Vec::new();
    let mut primary_key_field_name: Option<String> = None;

    for part_str in inner.split('/') {
        if part_str.is_empty() { continue; } 

        let parts: Vec<&str> = part_str.splitn(2, "::").collect();
        let name = parts[0].trim().to_string();
        if name.is_empty() {
            return Err(format!("Header field name cannot be empty in part: '{}'", part_str));
        }

        let mut type_info_str: Option<String> = None;
        let mut is_pk_from_type = false;

        if parts.len() == 2 {
            let type_part = parts[1].trim();
            if !type_part.is_empty() {
                type_info_str = Some(type_part.to_string());
                if (type_part == "index" || type_part == "sindex" || type_part == "gindex") && primary_key_field_name.is_none() {
                    primary_key_field_name = Some(name.clone());
                    is_pk_from_type = true;
                }
            }
        }
        
        headers.push(HeaderField {
            name,
            type_info: type_info_str,
            is_primary_key: is_pk_from_type, 
        });
    }
    if headers.is_empty() && !inner.is_empty() {
         return Err(format!("Header line parsed to empty headers, but was not empty: '{}'", line));
    }

    Ok((headers, primary_key_field_name))
}

pub fn parse_data_line(line_str: &str, headers: &[HeaderField], _header_map: &HashMap<String, usize>) -> Result<Row, String> {
    let mut parts = Vec::new();
    let mut current_part = String::new();
    let mut in_quotes = false;
    let mut parentheses_level = 0; // 新增：跟踪括号层级
    for char_code in line_str.chars() {
        match char_code {
            '"' => {
                in_quotes = !in_quotes;
                current_part.push(char_code);
            }
            '(' if !in_quotes => { // 新增：处理开括号
                parentheses_level += 1;
                current_part.push(char_code);
            }
            ')' if !in_quotes => { // 新增：处理闭括号
                if parentheses_level > 0 {
                    parentheses_level -= 1;
                }
                current_part.push(char_code);
            }
            ',' if !in_quotes && parentheses_level == 0 => { // 修改：增加括号层级判断
                parts.push(current_part.trim().to_string());
                current_part.clear();
            }
            _ => {
                current_part.push(char_code);
            }
        }
    }
    parts.push(current_part.trim().to_string()); // Add the last part

    if parts.len() > headers.len() && headers.len() > 0 { // headers.len() > 0 to allow schemaless tables
        return Err(format!("Data line has more parts ({}) than headers ({}): '{}'", parts.len(), headers.len(), line_str));
    }
    
    // If parts are fewer than headers, it might be due to trailing empty fields not captured by simple split.
    // The original code implicitly handled this by iterating up to parts.len() and then filling missing headers.
    // We need to ensure this behavior is preserved or correctly adapted.
    // The new splitting logic should correctly capture empty strings if they are meant to be fields.
    // For example, "a,,c" should result in ["a", "", "c"].
    // The current logic for `parts.push(current_part.trim().to_string());` might need adjustment if `current_part` is empty.
    // Let's test: if line is "a," -> parts: ["a", ""]. If "a,b," -> parts: ["a", "b", ""]. This seems correct.

    let mut fields = HashMap::new();
    for (i, part_str) in parts.iter().enumerate() {
        if i < headers.len() {
            let header = &headers[i];
            // parse_value_str will handle unquoting if the part_str is a quoted string
            let value = parse_value_str(part_str, header.type_info.as_deref());
            fields.insert(header.name.clone(), value);
        }
    }
    
    // Ensure all headers have a corresponding field, even if it's an empty string (or Null based on type)
    // This loop is crucial for data lines with fewer fields than headers.
    for (i, header) in headers.iter().enumerate() {
        if i >= parts.len() { // If a header doesn't have a corresponding part from the split
            if !fields.contains_key(&header.name) { // And it wasn't somehow added (shouldn't happen with i >= parts.len())
                 // Default to empty string, parse_value_str might convert to Null if appropriate for type
                fields.insert(header.name.clone(), parse_value_str("", header.type_info.as_deref()));
            }
        }
    }

    Ok(Row { fields })
}

// Helper function to split elements of a tuple string, respecting quotes and parentheses
fn split_tuple_elements(tuple_content: &str) -> Vec<String> {
    let mut elements = Vec::new();
    if tuple_content.is_empty() { // Handle empty tuple "()"
        return elements;
    }
    let mut current_element = String::new();
    let mut in_quotes = false;
    let mut parentheses_level = 0;

    for char_code in tuple_content.chars() {
        match char_code {
            '"' => {
                in_quotes = !in_quotes;
                current_element.push(char_code);
            }
            '(' if !in_quotes => {
                parentheses_level += 1;
                current_element.push(char_code);
            }
            ')' if !in_quotes => {
                if parentheses_level > 0 {
                    parentheses_level -= 1;
                }
                current_element.push(char_code);
            }
            ',' if !in_quotes && parentheses_level == 0 => {
                elements.push(current_element.trim().to_string());
                current_element.clear();
            }
            _ => {
                current_element.push(char_code);
            }
        }
    }
    // Add the last part. Handles cases like "(a)", "(a,b)", also if tuple_content was non-empty but resulted in no splits.
    elements.push(current_element.trim().to_string());
    elements
}

pub fn parse_value_str(s: &str, field_type_info: Option<&str>) -> Value {
    let trimmed_s = s.trim();

    if trimmed_s.is_empty() {
        // For consistency, if field_type_info suggests Null is possible for empty,
        // this could be Value::Null. But current parser returns Value::String("").
        // Let's keep Value::String("") for empty.
        return Value::String("".to_string());
    }

    // Handle "null" string, converting to Value::Null unless type is string
    if trimmed_s.eq_ignore_ascii_case("null") {
        if field_type_info.map_or(true, |t| t.to_lowercase() != "string") {
            return Value::Null;
        }
        // If type is "string", then "null" is the string "null"
    }

    // Attempt to parse as a tuple if it looks like one: ("elem1", "elem2", ...)
    if trimmed_s.starts_with('(') && trimmed_s.ends_with(')') && trimmed_s.len() >= 2 {
        let inner_content = &trimmed_s[1..trimmed_s.len() - 1];
        let elements_str = split_tuple_elements(inner_content);
        let mut values = Vec::new();
        for elem_s in elements_str {
            // Recursively parse each element. Pass None for type_info,
            // so it auto-detects or defaults to string for tuple elements.
            values.push(parse_value_str(&elem_s, None));
        }
        return Value::Tuple(values);
    }

    if let Some(type_info) = field_type_info {
        match type_info.to_lowercase().as_str() { // Normalize type_info for comparison
            "integer" => {
                if let Ok(i) = trimmed_s.parse::<i64>() {
                    return Value::Integer(i);
                } else {
                    eprintln!("[WARN] Failed to parse '{}' as integer, treating as string.", trimmed_s);
                    // Fall through to default string parsing if parse fails
                }
            }
            "boolean" => {
                if trimmed_s.eq_ignore_ascii_case("true") {
                    return Value::String("true".to_string());
                } else if trimmed_s.eq_ignore_ascii_case("false") {
                    return Value::String("false".to_string());
                } else {
                    eprintln!("[WARN] Failed to parse '{}' as boolean, treating as string.", trimmed_s);
                    // Fall through
                }
            }
            "string" => {
                // If type is explicitly string, return the trimmed string as is (quotes included if present).
                return Value::String(trimmed_s.to_string());
            }
            // Handle reference types like "type_name::key_value_string"
            // This check should be specific enough not to misinterpret other type_info.
            // Example: field_type_info could be "project_structure" (a custom type name)
            // or "reference::some_table".
            // The original code `ref_type if ref_type.contains("::")` was broad.
            // Let's assume if `type_info` contains "::", it's intended as a direct reference value itself.
            ref_val_type if ref_val_type.contains("::") => {
                 let parts: Vec<&str> = trimmed_s.splitn(2, "::").collect();
                 // This logic is for when the *value string itself* is "Type::Key",
                 // and field_type_info also indicates it's a reference.
                 // If field_type_info is "some_table_ref::some_table" and value is "Key1"
                 // then this part is not for that. This is for when value is "ActualType::ActualKey".
                 if parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                     let ref_type_name = parts[0].to_string();
                     let key_val_str = parts[1];
                     // For the key part of the reference, parse it as a basic value (int or unquoted string)
                     let key_val = if let Ok(i) = key_val_str.parse::<i64>() {
                         Box::new(Value::Integer(i))
                     } else {
                         let unquoted_key_val_str = if key_val_str.starts_with('"') && key_val_str.ends_with('"') && key_val_str.len() >= 2 {
                             key_val_str[1..key_val_str.len()-1].to_string()
                         } else {
                             key_val_str.to_string()
                         };
                         Box::new(Value::String(unquoted_key_val_str))
                     };
                     return Value::Reference { type_name: ref_type_name, key: key_val };
                 }
                 // If format is not "Type::Key" but type_info had "::", warn and fall through.
                 eprintln!("[WARN] Value '{}' with type_info '{}' expected format Type::Key for reference, treating as string.", trimmed_s, type_info);
            }
            // Add other known types like "date", "datetime" if they have specific parsing.
            // For now, they will fall through.
            _ => { /* Unknown type_info, fall through to default parsing */ }
        }
    }

    // Fallback parsing (if not a tuple, and no specific type_info matched or type_info was None)
    // This is also where elements from a tuple (parsed with type_info=None) will land.

    // 1. Try to parse as integer
    if let Ok(i) = trimmed_s.parse::<i64>() {
        return Value::Integer(i);
    }

    // 2. Try to parse as boolean "true" or "false" (case-insensitive)
    if trimmed_s.eq_ignore_ascii_case("true") {
        return Value::String("true".to_string()); // Consistent with "boolean" type_info parsing
    }
    if trimmed_s.eq_ignore_ascii_case("false") {
        return Value::String("false".to_string()); // Consistent with "boolean" type_info parsing
    }
    
    // 3. Handle quoted strings: if it's "\"content\"", store "content"
    if trimmed_s.starts_with('"') && trimmed_s.ends_with('"') && trimmed_s.len() >= 2 {
        return Value::String(trimmed_s[1..trimmed_s.len()-1].to_string());
    }

    // 4. Default to string as is (if not quoted, not int, not bool)
    Value::String(trimmed_s.to_string())
}