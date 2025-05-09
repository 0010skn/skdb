use std::collections::HashMap;

// --- Data Structures ---

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Integer(i64),
    Tuple(Vec<Value>),
    Reference { type_name: String, key: Box<Value> }, // For references like c::config
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderField {
    pub name: String, // Made public for parser module
    pub type_info: Option<String>, // e.g., "sindex", "config", "system", "index", "gindex"
    pub is_primary_key: bool, // True if this field is the key for index/gindex
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    // Using a HashMap for easier access by field name during querying
    pub fields: HashMap<String, Value>, // Made public for parser/query modules
}

impl Row {
    pub fn get_field_mut(&mut self, field_name: &str) -> Option<&mut Value> {
        self.fields.get_mut(field_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TableData {
    Sequential(Vec<Row>),
    Indexed(HashMap<String, Row>),      // Key is String representation of the primary key Value
    GroupedIndexed(HashMap<String, Vec<Row>>), // Key is String representation of the primary key Value
    RawLines(Vec<String>), // For tables defined by CopyStructure, data comes later without headers
}

impl TableData {
    // Helper to check if data is empty, useful for merging logic
    pub fn is_empty(&self) -> bool {
        match self {
            TableData::Sequential(rows) => rows.is_empty(),
            TableData::Indexed(map) => map.is_empty(),
            TableData::GroupedIndexed(map) => map.is_empty(),
            TableData::RawLines(lines) => lines.is_empty(),
        }
    }

    pub fn get_sequential_row(&self, index: usize) -> Option<&Row> {
        match self {
            TableData::Sequential(rows) => rows.get(index),
            _ => None, // Or panic/error if trying to get sequential from non-sequential
        }
    }

    pub fn get_sequential_row_mut(&mut self, index: usize) -> Option<&mut Row> {
        match self {
            TableData::Sequential(rows) => rows.get_mut(index),
            _ => None,
        }
    }

    pub fn get_indexed_row(&self, key: &str) -> Option<&Row> {
        match self {
            TableData::Indexed(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_indexed_row_mut(&mut self, key: &str) -> Option<&mut Row> {
        match self {
            TableData::Indexed(map) => map.get_mut(key),
            _ => None,
        }
    }
    
    pub fn get_grouped_rows(&self, key: &str) -> Option<&Vec<Row>> {
        match self {
            TableData::GroupedIndexed(map) => map.get(key),
            _ => None,
        }
    }

    pub fn get_grouped_rows_mut(&mut self, key: &str) -> Option<&mut Vec<Row>> {
        match self {
            TableData::GroupedIndexed(map) => map.get_mut(key),
            _ => None,
        }
    }

    pub fn add_sequential_row(&mut self, row: Row) -> Result<(), String> {
        match self {
            TableData::Sequential(rows) => {
                rows.push(row);
                Ok(())
            }
            TableData::RawLines(_) => Err("Cannot add parsed row to RawLines table data. Convert to Sequential first.".to_string()),
            _ => Err("Table is not sequential".to_string()),
        }
    }

    pub fn add_indexed_row(&mut self, key: String, row: Row) -> Result<(), String> {
        match self {
            TableData::Indexed(map) => {
                if map.contains_key(&key) {
                    return Err(format!("Key '{}' already exists in indexed table", key));
                }
                map.insert(key, row);
                Ok(())
            }
            TableData::RawLines(_) => Err("Cannot add parsed row to RawLines table data. Convert to Indexed first.".to_string()),
            _ => Err("Table is not indexed".to_string()),
        }
    }

    pub fn add_grouped_indexed_row(&mut self, key: String, row: Row) -> Result<(), String> {
        match self {
            TableData::GroupedIndexed(map) => {
                map.entry(key).or_insert_with(Vec::new).push(row);
                Ok(())
            }
            TableData::RawLines(_) => Err("Cannot add parsed row to RawLines table data. Convert to GroupedIndexed first.".to_string()),
            _ => Err("Table is not group-indexed".to_string()),
        }
    }

    pub fn len_sequential(&self) -> Option<usize> {
        match self {
            TableData::Sequential(rows) => Some(rows.len()),
            // RawLines doesn't have a direct sequential parsed row count until processed
            _ => None,
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub name: String, // Made public
    pub headers: Vec<HeaderField>, // Ordered list of headers, made public
    pub header_map: HashMap<String, usize>, // For quick lookup of header index by name, made public
    pub data: TableData, // Made public
    pub primary_key_field_name: Option<String>, // Name of the field used as primary key for indexed tables, made public
}

impl Table {
    // Helper to get the type of index for the table
    pub fn get_index_type(&self) -> Option<&str> {
        self.headers.iter().find_map(|h| {
            if h.is_primary_key {
                h.type_info.as_deref()
            } else {
                None
            }
        })
    }

    // Adds a new row to the table.
    // For ::sindex tables, it auto-assigns the next available integer ID.
    // For :index tables, the row must contain the primary key field.
    // For other table types or if pk is missing for :index, it might return an error.
    pub fn add_row(&mut self, mut new_row: Row) -> Result<(), String> {
        match self.get_index_type() {
            Some("sindex") => {
                // For sindex, the primary key is usually implicit or named 'id' or similar.
                // We need to find the primary key field name if explicitly defined,
                // or assume a convention if not (e.g., 'id').
                // For now, let's assume the parser ensures 'id' exists or is handled.
                // The actual ID value will be the current length of the sequential data.
                let next_id = self.data.len_sequential().unwrap_or(0); // Get current count for next ID
                
                // Find the sindex header field to ensure it exists.
                // The parser should have set this up.
                let pk_field_name = self.primary_key_field_name.as_deref()
                    .or_else(|| self.headers.iter().find(|h| h.type_info.as_deref() == Some("sindex")).map(|h| h.name.as_str()))
                    .ok_or_else(|| "sindex table missing primary key field definition".to_string())?;

                // Insert/update the ID field in the new row.
                new_row.fields.insert(pk_field_name.to_string(), Value::Integer(next_id as i64));
                
                self.data.add_sequential_row(new_row)
            }
            Some("index") => {
                let pk_field_name = self.primary_key_field_name.as_ref()
                    .ok_or_else(|| "Indexed table missing primary key field name".to_string())?;
                
                let pk_value = new_row.fields.get(pk_field_name)
                    .ok_or_else(|| format!("Missing primary key field '{}' in new row for indexed table '{}'", pk_field_name, self.name))?;
                
                let pk_str = match pk_value {
                    Value::String(s) => s.clone(),
                    Value::Integer(i) => i.to_string(),
                    // Other types might need specific string conversion or be disallowed as keys
                    _ => return Err(format!("Unsupported primary key type for field '{}' in table '{}'", pk_field_name, self.name)),
                };
                self.data.add_indexed_row(pk_str, new_row)
            }
            Some("gindex") => {
                 let pk_field_name = self.primary_key_field_name.as_ref()
                    .ok_or_else(|| "Group-indexed table missing primary key field name".to_string())?;
                
                let pk_value = new_row.fields.get(pk_field_name)
                    .ok_or_else(|| format!("Missing primary key field '{}' in new row for group-indexed table '{}'", pk_field_name, self.name))?;

                let pk_str = match pk_value {
                    Value::String(s) => s.clone(),
                    Value::Integer(i) => i.to_string(),
                    _ => return Err(format!("Unsupported primary key type for field '{}' in group-indexed table '{}'", pk_field_name, self.name)),
                };
                self.data.add_grouped_indexed_row(pk_str, new_row)
            }
            Some(other_type) => Err(format!("Adding rows to table type '{}' is not yet supported", other_type)),
            None => { // No primary key, assume it's a simple sequential table without a special index type
                 // This case might need clarification: if no ::sindex, :index, etc. is defined,
                 // how should .add() behave? For now, treat as sequential if no PK is defined.
                 // However, the parser usually assigns an index type.
                 // If it's truly a "typeless" table, it might behave like sequential.
                 // Let's assume for now that if it's not explicitly sindex, index, or gindex,
                 // but is sequential in structure, we can add to it.
                 match &mut self.data {
                    TableData::Sequential(_) => self.data.add_sequential_row(new_row),
                    _ => Err("Cannot add row to a non-sequential table without a defined sindex/index type.".to_string())
                 }
            }
        }
    }
}

pub type DslRoot = HashMap<String, Table>;