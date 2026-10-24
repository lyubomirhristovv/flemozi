pub fn klipy_api_key() -> Option<String> {
    std::env::var("KLIPY_API_KEY").ok()
}
