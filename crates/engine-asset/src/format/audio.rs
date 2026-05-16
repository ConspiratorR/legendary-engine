pub fn load_audio(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("Failed to load audio '{}': {}", path, e))
}
