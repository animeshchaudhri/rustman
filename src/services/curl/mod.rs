mod generator;
mod parser;

pub use generator::{GenerateCurlInput, KvPair};
pub use parser::ParsedCurl;

pub fn parse(cmd: &str) -> ParsedCurl {
    parser::parse(cmd)
}

pub fn generate(input: &GenerateCurlInput) -> String {
    generator::generate(input)
}
