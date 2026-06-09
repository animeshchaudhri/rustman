mod generator;
mod parser;

pub use generator::{GenerateCurlInput, KvPair};
pub use parser::{CurlForm, ParsedCurl};

pub fn parse(cmd: &str) -> ParsedCurl {
    parser::parse(cmd)
}

pub(crate) fn tokenize(s: &str) -> Vec<String> {
    parser::shell_tokenize(s)
}

pub fn generate(input: &GenerateCurlInput) -> String {
    generator::generate(input)
}
