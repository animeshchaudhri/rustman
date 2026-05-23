mod generator;
mod parser;

pub use generator::GenerateCurlInput;
pub use parser::ParsedCurl;

#[tauri::command]
pub fn parse_curl(cmd: String) -> ParsedCurl {
    parser::parse(&cmd)
}

#[tauri::command]
pub fn generate_curl(input: GenerateCurlInput) -> String {
    generator::generate(&input)
}
