use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaskError {
    #[error("join error")]
    ThreadJoin,    
    #[error("other error - {0}")]
    Other(String),
}