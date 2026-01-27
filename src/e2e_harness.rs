pub fn is_ready_line(line: &str) -> bool {
    line.contains("listening on http://")
}

pub fn build_rpc_body(
    id: u64,
    method: &str,
    params: Option<serde_json::Value>,
) -> String {
    let request = crate::rpc::RpcRequest {
        id,
        method: method.to_string(),
        params,
    };
    serde_json::to_string(&request).unwrap_or_default()
}

pub fn parse_rpc_response(
    input: &str,
) -> Result<crate::rpc::RpcResponse, serde_json::Error> {
    serde_json::from_str(input)
}
