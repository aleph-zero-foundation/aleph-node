use crate::session::TestSessionKeys;
use common::Connection;
use serde_json::{json, Value};
use substrate_api_client::{ApiResult, FromHexString};

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

pub fn get_author_rotate_keys(connection: &Connection) -> ApiResult<Option<TestSessionKeys>> {
    let jsonreq = author_rotate_keys();
    let p = connection.get_request(jsonreq)?;
    Ok(p.map(|keys| {
        let bytes: Vec<u8> =
            FromHexString::from_hex(keys).expect("String hex-encoded session cannot be decoded");
        TestSessionKeys::from(bytes)
    }))
}
