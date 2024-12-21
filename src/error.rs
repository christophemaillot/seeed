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

    #[error("template error {0}")]
    Template(#[from] minijinja::Error),
}
