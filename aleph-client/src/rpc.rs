use crate::{Connection, SessionKeys};
use serde_json::{json, Value};

fn json_req(method: &str, params: Value, id: u32) -> Value {
    json!({
        "method": method,
        "params": params,
        "jsonrpc": "2.0",
        "id": id.to_string(),
    })
}

pub fn author_rotate_keys() -> Value {
    json_req("author_rotateKeys", Value::Null, 1)
}

pub fn rotate_keys_base<F, R>(
    connection: &Connection,
    rpc_result_mapper: F,
) -> Result<R, &'static str>
where
    F: Fn(String) -> Option<R>,
{
    match connection.get_request(author_rotate_keys()) {
        Ok(maybe_keys) => match maybe_keys {
            Some(keys) => match rpc_result_mapper(keys) {
                Some(keys) => Ok(keys),
                None => Err("Failed to parse keys from string result"),
            },
            None => Err("Failed to retrieve keys from chain"),
        },
        Err(_) => Err("Connection does not work"),
    }
}

pub fn rotate_keys(connection: &Connection) -> Result<SessionKeys, &'static str> {
    rotate_keys_base(connection, |keys| match SessionKeys::try_from(keys) {
        Ok(keys) => Some(keys),
        Err(_) => None,
    })
}

pub fn rotate_keys_raw_result(connection: &Connection) -> Result<String, &'static str> {
    // we need to escape two characters from RPC result which is escaped quote
    rotate_keys_base(connection, |keys| Some(keys.trim_matches('\"').to_string()))
}
