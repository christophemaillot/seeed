use thiserror::Error;

#[derive(Debug, Error)]
pub enum SeeedError {

    #[error("IO error while reading file")]
    IoError(#[from] std::io::Error),
    
    #[error("SSH error")]
    SshError(#[from] ssh2::Error),

    #[error("Incorrect target specified")]
    BadTarget,

    #[error("unknown function invocation")]
    UnknownFunction(),

    #[error("wrong number of arguments, expected {0}, got {1}")]
    WrongArgCount(usize, usize),

    #[error("bad argument to function call")]
    BadArgType(String),

    #[error("bad argument to function call")]
    BadArgument(&'static str),

    #[error("undefined variable {0}")]
    UndefinedVar(String),

    #[error("Parsing error at line {line}:{col}\n{line_content}\n{pointer}\n{message}")]
    ParseError {
        message: String,
        line: usize,
        col: usize,
        line_content: String,
        pointer: String,
    },
    
    #[error("template error {0}")]
    Template(#[from] minijinja::Error),

    #[error("can only iterate over an array")]
    IterateOverArray,

    #[error("UTF-8 conversion error")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    
    #[error("UTF-8 conversion error")]
    Utf8ErrorSlice(#[from] std::str::Utf8Error),

    #[error("Regex error")]
    RegexError(#[from] regex::Error),

    #[error("SSH error: {0}")]
    GenericSshError(String),
    
    #[error("Channel communication error")]
    ChannelError(String),
}
