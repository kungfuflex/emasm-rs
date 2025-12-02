use thiserror::Error;

#[derive(Debug, Error)]
pub enum AssemblerError {
    #[error("Unknown opcode: {0}")]
    UnknownOpcode(String),
    
    #[error("Label not found: {0}")]
    LabelNotFound(String),
    
    #[error("Invalid hex literal: {0}")]
    InvalidHexLiteral(String),
    
    #[error("Integer overflow: value too large for PUSH instruction")]
    IntegerOverflow,
    
    #[error("Invalid bytes segment: {0}")]
    InvalidBytesSegment(String),
    
    #[error("Circular label dependency detected")]
    CircularDependency,
    
    #[error("Invalid placeholder index: {0}")]
    InvalidPlaceholder(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmElement {
    Opcode(String),
    Literal(Vec<u8>),
    Label(String),
    Segment(String, Vec<AsmElement>),
    BytesSegment(String, Vec<u8>),
    BytesPtr(String),
    BytesSize(String),
    Placeholder(usize),
}

#[derive(Debug, Clone)]
pub struct LabelInfo {
    pub offset: usize,
    pub size_estimate: usize,
}

#[derive(Debug, Clone)]
pub struct BytesInfo {
    pub offset: usize,
    pub size: usize,
}
