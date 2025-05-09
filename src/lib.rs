pub mod web_server;
pub mod api;
pub mod transaction;
pub mod storage;
pub mod query_processor;
pub mod log;

// DSL Parser Modules
pub mod structs;
pub mod parser;
pub mod query;

// Public API for the DSL Parser
pub use structs::{Value, DslRoot, Table, Row, HeaderField, TableData}; // Export main structs needed by users
pub use parser::parse_dsl_input;
pub use parser::DslStatement;
pub use query::{execute_query, execute_update, execute_add, execute_pack};


pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    // TODO: Add tests for DSL parser once main.rs is cleaned up and uses the library.
    // For example:
    // #[test]
    // fn test_dsl_parsing_and_query() {
    //     let dsl_input = "user:\n/id::sindex/name/\n0,test_user\n~";
    //     let root = parse_dsl(dsl_input).expect("DSL parsing failed");
    //     let name_val = execute_query(&root, "#.user[0].name");
    //     assert_eq!(name_val, Some(&Value::String("test_user".to_string())));
    // }
}
