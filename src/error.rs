#[derive(Debug)]
pub enum PatchError {
    ParseError(String),
    IOError(std::io::Error),
    Unknown,
}
