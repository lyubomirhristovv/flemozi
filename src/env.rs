include!(concat!(env!("OUT_DIR"), "/env_vars.rs"));

pub fn klipy_api_key() -> Option<String> {
    get("KLIPY_API_KEY").or_else(|| std::env::var("KLIPY_API_KEY").ok())
}
