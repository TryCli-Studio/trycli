//  CONFIGURATION HELPERS
pub fn api_base() -> &'static str {
    option_env!("API_URL").unwrap_or("http://localhost:3000")
}

pub fn ws_base() -> &'static str {
    option_env!("WS_URL").unwrap_or("ws://localhost:3000")
}
