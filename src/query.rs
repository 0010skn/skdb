use std::collections::HashMap;
use crate::structs::{Value, Row, DslRoot, TableData, Table};
use crate::parser::{value_to_string_key, parse_value_str};
use crate::structs::HeaderField;

fn is_primitive_or_special_type(type_name: &str) -> bool {
    matches!(type_name.to_lowercase().as_str(),
        "integer" | "string" | "boolean" | "date" | "datetime" | // Known primitive types
        "sindex" | "index" | "gindex" | "config" | "system" | // Known special directive types
        _ if type_name.contains("::") // Covers references like "any::table" or specific type hints
    )
}

fn tokenize_query_path(query_path_str: &str) -> Result<Vec<String>, String> {
    if !query_path_str.starts_with("#.") {
        return Err("Query must start with #.".to_string());
    }
    let path = &query_path_str[2..];
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut in_bracket = false; 

    for ch in path.chars() {
        match ch {
            '.' => {
                if in_bracket { 
                    current_token.push(ch);
                } else { 
                    if !current_token.is_empty() {
                        tokens.push(current_token);
                        current_token = String::new();
                    }
                }
            }
            '[' => {
                if in_bracket { return Err("Nested brackets '[[...]]' are not supported in query path.".to_string()); }
                if !current_token.is_empty() { 
                    tokens.push(current_token);
                    current_token = String::new();
                }
                in_bracket = true;
                current_token.push(ch); 
            }
            ']' => {
                if !in_bracket { return Err("Unmatched ']' in query path.".to_string()); }
                current_token.push(ch); 
                tokens.push(current_token); 
                current_token = String::new();
                in_bracket = false;
            }
            _ => {
                current_token.push(ch);
            }
        }
    }
    if in_bracket { return Err("Unclosed '[' in query path.".to_string()); }
    if !current_token.is_empty() { 
        tokens.push(current_token);
    }
    Ok(tokens)
}

pub fn execute_query<'a>(root: &'a DslRoot, query_path_str: &str) -> Option<&'a Value> {
    let parts = match tokenize_query_path(query_path_str) {
        Ok(p) => p,
        Err(_) => return None,
    };

    if parts.is_empty() { return None; }

    let mut current_table_name: Option<String> = None;
    let mut current_row_context: Option<&'a Row> = None;
    let mut current_value_context: Option<&'a Value> = None;
    let mut current_gindexed_rows_context: Option<&'a Vec<Row>> = None;
    let mut current_tuple_structure_name: Option<String> = None; // For knowing the structure of a Value::Tuple

    for (idx, part_str_outer) in parts.iter().enumerate() {
        let part_str = part_str_outer.as_str(); // Use &str for find and slicing

        if idx == 0 {
            let mut table_name_slice = part_str;
            let mut key_to_use: Option<&str> = None;

            if let Some(brace_open_pos) = part_str.find('{') {
                if let Some(brace_close_pos) = part_str.rfind('}') {
                    if brace_open_pos < brace_close_pos && brace_open_pos > 0 { // Ensure table name part exists before '{'
                        table_name_slice = &part_str[..brace_open_pos];
                        key_to_use = Some(&part_str[brace_open_pos + 1..brace_close_pos]);
                    } else { return None; /* malformed {} or empty table name */ }
                } else { return None; /* malformed {} */ }
            }
            
            current_table_name = Some(table_name_slice.to_string());
            let table = match root.get(table_name_slice) {
                Some(t) => t,
                None => { /* println!("[DEBUG] Table '{}' not found in root at idx 0", table_name_slice); */ return None; }
            };

            if let Some(key_str) = key_to_use {
                match &table.data {
                    TableData::Indexed(map) => {
                        current_row_context = map.get(key_str);
                        if current_row_context.is_none() {
                            // Key not found, but if this is the *only* part (e.g. #.table{key})
                            // and more parts follow in the query string (e.g. .field), then it's an error.
                            // If no more parts follow, returning None for current_value_context is correct.
                            // However, if a field is expected next, not finding the row means the path is invalid.
                            if parts.len() > idx + 1 { // Check if more parts are expected
                                // println!("[DEBUG] Key '{}' not found in indexed table '{}' but more parts follow.", key_str, table_name_slice);
                                return None;
                            }
                        }
                    }
                    _ => { /* println!("[DEBUG] {{key}} syntax used on non-indexed table '{}'", table_name_slice); */ return None; }
                }
            }
            current_value_context = None;
            current_gindexed_rows_context = None;
            continue;
        }

        // For subsequent parts (idx > 0)
        // current_table_name should be set from idx == 0
        let table_name_to_use = match current_table_name.as_ref() {
            Some(name) => name,
            None => { /* println!("[DEBUG] current_table_name is None at idx > 0"); */ return None; } // Should have been set
        };
        
        // table_to_query is needed if current_row_context is not already set (e.g. by {key} or by previous [idx])
        // or if we need to access table.data for a new [idx] lookup on the table itself.
        let table_to_query = match root.get(table_name_to_use) {
            Some(t) => t,
            None => { /* println!("[DEBUG] Table '{}' (derived from current_table_name) not found at idx > 0", table_name_to_use); */ return None; }
        };

        if part_str.starts_with('[') && part_str.ends_with(']') {
            let key_or_idx_val = &part_str[1..part_str.len() - 1];
            if current_value_context.is_some() { 
                match current_value_context.unwrap() {
                    Value::Tuple(values) => {
                        if let Ok(arr_idx) = key_or_idx_val.parse::<usize>() {
                            current_value_context = values.get(arr_idx);
                            current_row_context = None; 
                            current_gindexed_rows_context = None;
                        } else { return None; }
                    }
                    _ => return None, 
                }
            } else if current_gindexed_rows_context.is_some() { 
                 if let Ok(arr_idx) = key_or_idx_val.parse::<usize>() {
                     current_row_context = current_gindexed_rows_context.unwrap().get(arr_idx);
                     current_value_context = None; 
                     current_gindexed_rows_context = None; 
                 } else { return None; }
            } else { 
                match &table_to_query.data {
                    TableData::Sequential(rows) => {
                        if let Ok(arr_idx) = key_or_idx_val.parse::<usize>() {
                            current_row_context = rows.get(arr_idx);
                            current_value_context = None;
                        } else { return None; }
                    }
                    TableData::Indexed(map) => {
                        current_row_context = map.get(key_or_idx_val);
                        current_value_context = None;
                    }
                    TableData::GroupedIndexed(_) => return None,
                    TableData::RawLines(_) => return None,
                }
            }
            if current_row_context.is_none() && current_value_context.is_none() && idx < parts.len() -1 {
                return None;
            }
        } else { 
            let field_val_opt: Option<&'a Value>; 

            if current_row_context.is_some() { 
                let row = current_row_context.unwrap();
                field_val_opt = row.fields.get(part_str);
                current_value_context = field_val_opt; // Set value context first
                current_tuple_structure_name = None;   // Reset structure name

                if let Some(val) = field_val_opt {
                    // Determine if this value (now current_value_context) is a tuple with a known structure
                    if let Value::Tuple(_) = val {
                        let original_table_name_for_header = current_table_name.as_ref().ok_or_else(|| { /* Should not happen */ None::<&Value> }).unwrap();
                        if let Some(original_table_def) = root.get(original_table_name_for_header) {
                            if let Some(header_field) = original_table_def.headers.iter().find(|h| h.name == part_str) {
                                if let Some(type_info) = &header_field.type_info {
                                    if root.contains_key(type_info) && !is_primitive_or_special_type(type_info) {
                                        current_tuple_structure_name = Some(type_info.clone());
                                    }
                                }
                            }
                        }
                    }

                    // Handle if the field itself is a reference
                    if let Value::Reference { type_name: ref_table_str, key: ref_key_val_boxed } = val {
                        let ref_key_val = &**ref_key_val_boxed;
                        current_table_name = Some(ref_table_str.clone()); // Update current_table_name for next part
                        let referenced_table = match root.get(ref_table_str) {
                            Some(rt) => rt,
                            None => return None,
                        };

                        match &referenced_table.data {
                            TableData::Sequential(rows) => {
                                let index_opt = match ref_key_val {
                                    Value::Integer(i) => Some(*i as usize),
                                    Value::Tuple(vals) if vals.len() == 1 => {
                                        if let Value::Integer(i_inner) = vals[0] { Some(i_inner as usize) } else { None }
                                    }
                                    _ => None,
                                };
                                current_row_context = index_opt.and_then(|index| rows.get(index));
                            }
                            TableData::Indexed(map) => {
                                let key_to_lookup_res = match ref_key_val {
                                    Value::Tuple(vals) if vals.len() == 1 => value_to_string_key(&vals[0]),
                                    _ => value_to_string_key(ref_key_val),
                                };
                                current_row_context = key_to_lookup_res.ok().and_then(|key_str| map.get(&key_str));
                            }
                            TableData::GroupedIndexed(_) => return None,
                            TableData::RawLines(_) => return None,
                        }
                        current_value_context = None; // After dereferencing, context is now a row or None
                        current_tuple_structure_name = None; // Reset as we are now in a new row context
                        if current_row_context.is_none() { return None; }
                    } else {
                        // It's not a reference, so we are now focused on 'val'.
                        // The row context is no longer the primary focus for the next part of the path.
                        current_row_context = None;
                    }
                    // current_value_context is 'val' (field_val_opt).
                    // current_tuple_structure_name might be set if 'val' is a Tuple with type_info.
                } else { // Field was not found in the row
                    current_row_context = None; // No row context if field not found
                    current_value_context = None; // Ensure value context is also None
                }
            } else if let Some(val_ctx) = current_value_context { // current_row_context is None, try to navigate current_value_context
                match val_ctx {
                    Value::Tuple(elements) => {
                        if let Some(structure_name) = &current_tuple_structure_name {
                            if let Some(structure_table_def) = root.get(structure_name) {
                                if let Some(field_idx_in_tuple) = structure_table_def.header_map.get(part_str) {
                                    current_value_context = elements.get(*field_idx_in_tuple);
                                    current_tuple_structure_name = None; // Reset, re-evaluate if new value is tuple
                                    current_row_context = None; // Still navigating a value, not a row
                                    current_gindexed_rows_context = None;

                                    // If the new current_value_context is also a tuple, determine its structure name
                                    if let Some(new_val) = current_value_context {
                                        if let Value::Tuple(_) = new_val {
                                            if let Some(sub_header) = structure_table_def.headers.get(*field_idx_in_tuple) {
                                                if let Some(sub_type_info) = &sub_header.type_info {
                                                    if root.contains_key(sub_type_info) && !is_primitive_or_special_type(sub_type_info) {
                                                        current_tuple_structure_name = Some(sub_type_info.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else { return None; /* field not in expected structure */ }
                            } else { return None; /* expected structure definition not found */ }
                        } else { return None; /* tuple, but no known structure for it */ }
                    }
                    _ => { /* trying to do .field on a non-tuple scalar value */ return None; }
                }
            } else { // No current_row_context and no current_value_context (e.g. #.table.field)
                match &table_to_query.data {
                    TableData::Sequential(rows) => {
                        if rows.len() >= 1 {
                            current_row_context = rows.get(0); 
                            if current_row_context.is_some() {
                                field_val_opt = current_row_context.unwrap().fields.get(part_str); 
                                if let Some(Value::Reference{type_name: ref_table_str, key: ref_key_val_boxed}) = field_val_opt {
                                    let ref_key_val = &**ref_key_val_boxed;
                                    current_table_name = Some(ref_table_str.clone());
                                    let referenced_table = root.get(ref_table_str)?; 
                                    
                                    match &referenced_table.data {
                                        TableData::Sequential(s_rows) => { 
                                            let index_opt = match ref_key_val {
                                                Value::Integer(i) => Some(*i as usize),
                                                Value::Tuple(vals) if vals.len() == 1 => {
                                                    if let Value::Integer(i_inner) = vals[0] { Some(i_inner as usize) } else { None }
                                                }
                                                _ => None,
                                            };
                                            match index_opt {
                                                Some(index) => current_row_context = s_rows.get(index),
                                                None => return None,
                                            }
                                        }
                                        TableData::Indexed(map) => {
                                            let key_to_lookup_res = match ref_key_val {
                                                Value::Tuple(vals) if vals.len() == 1 => value_to_string_key(&vals[0]),
                                                _ => value_to_string_key(ref_key_val),
                                            };
                                            match key_to_lookup_res {
                                                Ok(key_str) => current_row_context = map.get(&key_str),
                                                Err(_) => { return None; }
                                            }
                                        }
                                        TableData::GroupedIndexed(_) => return None, 
                                        TableData::RawLines(_) => return None, 
                                    }
                                    current_value_context = None;
                                    if current_row_context.is_none() { return None; }
                                } else {
                                    current_value_context = field_val_opt;
                                }
                            } else { return None; }
                        } else { return None; }
                    }
                    TableData::Indexed(map) => { 
                        current_row_context = map.get(part_str);
                        current_value_context = None; 
                        if current_row_context.is_none() && idx < parts.len() -1 { return None; }
                    }
                    TableData::GroupedIndexed(map) => { 
                        current_gindexed_rows_context = map.get(part_str);
                        current_row_context = None; 
                        current_value_context = None;
                        if current_gindexed_rows_context.is_none() && idx < parts.len() -1 { return None; }
                    }
                    TableData::RawLines(_) => return None,
                }
            }
            if idx < parts.len() - 1 && current_row_context.is_none() && current_value_context.is_none() && current_gindexed_rows_context.is_none() {
                return None;
            }
        }
    } 
    current_value_context
}

fn find_value_mut<'a>(
    root: &'a mut DslRoot,
    query_path_str_no_prefix: &str, 
) -> Result<(&'a mut Value, Option<String>), String> { 
    let query_path_str = format!("#.{}", query_path_str_no_prefix); 
    let parts = tokenize_query_path(&query_path_str)?;

    if parts.is_empty() {
        return Err("Query path is empty after tokenization.".to_string());
    }

    let mut _current_table_name: Option<String> = None; 
    let mut _current_table_primary_key_type: Option<String> = None; 
    let mut final_field_name_for_type_lookup: Option<String> = None;

    if parts.len() < 1 { 
        return Err("Query path is too short.".to_string());
    }
    
    let table_name_from_path = &parts[0];
    let table = root.get_mut(table_name_from_path)
        .ok_or_else(|| format!("Table '{}' not found.", table_name_from_path))?;
    
    _current_table_name = Some(table.name.clone());
    if let Some(pk_name) = &table.primary_key_field_name {
        if let Some(header) = table.headers.iter().find(|h| &h.name == pk_name) {
            _current_table_primary_key_type = header.type_info.clone();
        }
    }

    // let mut _current_target: &mut Value; // Unused
    // let _temp_row_storage: Option<Row> = None; // Unused and mutability not needed

    if parts.len() == 1 {
        return Err("Query path only specifies a table, not a field or value.".to_string());
    }

    let mut part_idx = 1;
    let target_row: &mut Row; 

    let part1_str = &parts[part_idx];
    if part1_str.starts_with('[') && part1_str.ends_with(']') { 
        let key_or_idx_val_str = &part1_str[1..part1_str.len() - 1];
        match &mut table.data {
            TableData::Sequential(rows) => {
                let row_idx = key_or_idx_val_str.parse::<usize>()
                    .map_err(|_| format!("Invalid sequential index: {}", key_or_idx_val_str))?;
                target_row = rows.get_mut(row_idx)
                    .ok_or_else(|| format!("Index {} out of bounds for table '{}'", row_idx, table.name))?;
            }
            TableData::Indexed(map) => {
                if !map.contains_key(key_or_idx_val_str) {
                    return Err(format!("Key '{}' not found in indexed table '{}'", key_or_idx_val_str, table.name));
                }
                if parts.len() == part_idx + 1 { 
                     return Err(format!("Cannot update an entire row directly using path '{}'. Specify a field.", query_path_str));
                }
                target_row = map.get_mut(key_or_idx_val_str)
                                .ok_or_else(|| format!("Key '{}' not found in indexed table '{}'", key_or_idx_val_str, table.name))?;
            }
            TableData::GroupedIndexed(_map) => { 
                return Err(format!("Updating GroupedIndexed table data directly via find_value_mut is not yet fully supported. Path: {}", query_path_str));
            }
            TableData::RawLines(_) => {
                return Err(format!("Cannot get mutable row from RawLines table '{}'", table.name));
            }
        }
        part_idx += 1;
    } else { 
        match &mut table.data {
            TableData::Sequential(rows) => {
                if rows.len() == 1 {
                    target_row = rows.get_mut(0).unwrap(); 
                } else if rows.is_empty() && table.headers.iter().any(|h| h.name == *part1_str) {
                    return Err(format!("Cannot update field '{}' in empty (but presumed single-row) table '{}'. Add row first.", part1_str, table.name));
                }
                else {
                    return Err(format!("Direct field access on table '{}' is only supported if it's a single-row sequential table.", table.name));
                }
            }
            TableData::RawLines(_) => {
                 return Err(format!("Direct field access on RawLines table '{}' is not supported.", table.name));
            }
            _ => return Err(format!("Direct field access on table '{}' is only supported for single-row sequential tables (or not applicable for this type).", table.name)),
        }
    }

    if part_idx >= parts.len() {
        return Err("Query path does not specify a field to update.".to_string());
    }

    let field_name_str = &parts[part_idx];
    final_field_name_for_type_lookup = Some(field_name_str.clone());
    
    let field_to_update = target_row.get_field_mut(field_name_str)
        .ok_or_else(|| format!("Field '{}' not found in table '{}'", field_name_str, table.name))?;

    if let Value::Reference { .. } = field_to_update {
        return Err(format!("Updating through a reference field ('{}' in table '{}') is not directly supported. Update the target value directly.", field_name_str, table.name));
    }
    
    part_idx += 1; 

    let mut final_target_value = field_to_update;

    if part_idx < parts.len() { 
        if !(parts[part_idx].starts_with('[') && parts[part_idx].ends_with(']')) {
            return Err(format!("Unexpected path segment after field: {}. Expected tuple index like '[0]'.", parts[part_idx]));
        }
        let tuple_idx_str = &parts[part_idx][1..parts[part_idx].len()-1];
        let tuple_idx = tuple_idx_str.parse::<usize>()
            .map_err(|_| format!("Invalid tuple index: {}", tuple_idx_str))?;

        match final_target_value {
            Value::Tuple(values) => {
                final_target_value = values.get_mut(tuple_idx)
                    .ok_or_else(|| format!("Tuple index {} out of bounds for field '{}'", tuple_idx, field_name_str))?;
            }
            _ => return Err(format!("Field '{}' is not a tuple, cannot index with '{}'", field_name_str, parts[part_idx])),
        }
        part_idx += 1;
    }

    if part_idx < parts.len() {
        return Err(format!("Extra path segments found after value selection: {:?}", &parts[part_idx..]));
    }
    
    let field_type_info: Option<String> = table.headers.iter()
        .find(|h| final_field_name_for_type_lookup.as_ref().map_or(false, |name| &h.name == name))
        .and_then(|h| h.type_info.clone());

    Ok((final_target_value, field_type_info))
}


pub fn execute_update(
    root: &mut DslRoot,
    path_str: &str,    
    value_str: &str, 
) -> Result<(), String> {
    let (target_value, field_type_info) = find_value_mut(root, path_str)?; 
    
    let new_value = parse_value_str(value_str, field_type_info.as_deref());

    match (target_value.clone(), &new_value) { 
        (Value::Integer(_), Value::String(s)) => {
            if let Ok(parsed_int) = s.parse::<i64>() {
                *target_value = Value::Integer(parsed_int);
            } else if field_type_info.is_none() || field_type_info.as_deref() == Some("string") { 
                 *target_value = new_value; 
            }
            else {
                return Err(format!("Type mismatch: Expected an integer for path '{}', got string '{}' which is not a valid integer.", path_str, s));
            }
        }
        (Value::String(_), Value::Integer(i)) => {
            if field_type_info.as_deref() == Some("integer") { 
                *target_value = new_value;
            } else {
                *target_value = Value::String(i.to_string());
            }
        }
        _ => {
            if std::mem::discriminant(target_value) != std::mem::discriminant(&new_value) &&
               !matches!((&*target_value, &new_value), (Value::Null, _) | (_, Value::Null)) 
            {
                if field_type_info.is_some() && new_value != Value::Null {
                    let target_type_name = value_type_to_string(target_value);
                    let new_value_type_name = value_type_to_string(&new_value);
                    if target_type_name != new_value_type_name { 
                        return Err(format!(
                            "Type mismatch for path '{}': target is {}, but received value is {} ('{}')",
                            path_str, target_type_name, new_value_type_name, value_str
                        ));
                    }
                } else {
                     *target_value = new_value; 
                }
            } else {
                 *target_value = new_value; 
            }
        }
    }

    Ok(())
}

fn value_type_to_string(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "String",
        Value::Integer(_) => "Integer",
        Value::Tuple(_) => "Tuple",
        Value::Reference { .. } => "Reference",
        Value::Null => "Null",
    }
}

pub fn execute_add(root: &mut DslRoot, table_name_str: &str) -> Result<(), String> {
    let table = root.get_mut(table_name_str)
        .ok_or_else(|| format!("Table '{}' not found for add operation.", table_name_str))?;

    let mut new_row = Row { fields: HashMap::new() };
    for header in &table.headers {
        new_row.fields.insert(header.name.clone(), Value::Null);
    }

    table.add_row(new_row)
}


// --- Pack Operation ---

fn serialize_value(value: &Value, header_map: &HashMap<String, usize>, headers: &[HeaderField]) -> String {
    match value {
        Value::String(s) => {
            if s.contains(',') || s.contains('(') || s.contains(')') || s.contains('\'') || s.contains(' ') || s.is_empty() {
                format!("'{}'", s.replace('\'', "''")) 
            } else {
                s.clone()
            }
        }
        Value::Integer(i) => i.to_string(),
        Value::Tuple(items) => {
            let item_strs: Vec<String> = items
                .iter()
                .map(|item| serialize_value(item, header_map, headers)) 
                .collect();
            format!("({})", item_strs.join(","))
        }
        Value::Reference { type_name: _type_name, key } => { 
            let key_str = serialize_value(key, header_map, headers); 
            format!("({})", key_str) 
        }
        Value::Null => "".to_string(),  
    }
}

pub fn execute_pack(root: &DslRoot, table_names: &[String]) -> Result<String, String> {
    let mut packed_strings = Vec::new();

    for table_name_to_pack in table_names {
        let table = root.get(table_name_to_pack)
            .ok_or_else(|| format!("Table '{}' not found for packing.", table_name_to_pack))?;

        let mut table_content = String::new();

        table_content.push_str(&format!("{}:\n", table.name));

        if !table.headers.is_empty() {
            let header_parts: Vec<String> = table.headers.iter().map(|h| {
                let mut part = h.name.clone();
                if let Some(type_info) = &h.type_info {
                    if h.is_primary_key && (type_info == "index" || type_info == "gindex" || type_info == "sindex") {
                        part = format!("{}:{}", h.name, type_info);
                    } else {
                        part = format!("{}::{}", h.name, type_info);
                    }
                }
                part
            }).collect();
            table_content.push_str(&format!("/{}/\n", header_parts.join("/")));
        } else {
             table_content.push_str("//\n");
        }

        match &table.data {
            TableData::Sequential(rows) => {
                for row in rows {
                    let row_values: Vec<String> = table.headers.iter().map(|header_field| {
                        row.fields.get(&header_field.name)
                            .map_or("".to_string(), |value| serialize_value(value, &table.header_map, &table.headers))
                    }).collect();
                    table_content.push_str(&format!("{}\n", row_values.join(",")));
                }
            }
            TableData::RawLines(_) => {
                return Err(format!("Cannot pack table '{}': data is in RawLines format. Process table first.", table.name));
            }
            TableData::Indexed(map) => {
                let mut sorted_keys: Vec<&String> = map.keys().collect();
                sorted_keys.sort(); 

                for key in sorted_keys {
                    let row = &map[key];
                    let row_values: Vec<String> = table.headers.iter().map(|header_field| {
                        row.fields.get(&header_field.name)
                            .map_or("".to_string(), |value| serialize_value(value, &table.header_map, &table.headers))
                    }).collect();
                    table_content.push_str(&format!("{}\n", row_values.join(",")));
                }
            }
            TableData::GroupedIndexed(map) => {
                let mut sorted_group_keys: Vec<&String> = map.keys().collect();
                sorted_group_keys.sort();

                for group_key in sorted_group_keys {
                    let rows_in_group = &map[group_key];
                    for row in rows_in_group {
                        let row_values: Vec<String> = table.headers.iter().map(|header_field| {
                            row.fields.get(&header_field.name)
                                .map_or("".to_string(), |value| serialize_value(value, &table.header_map, &table.headers))
                        }).collect();
                        table_content.push_str(&format!("{}\n", row_values.join(",")));
                    }
                }
            }
        }
        packed_strings.push(table_content.trim_end_matches('\n').to_string()); 
    }

    Ok(packed_strings.join("\n~\n")) 
}