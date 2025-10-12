use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkThreadError {
    #[error("join error")]
    ThreadJoin, 
    #[error("thread add error : {0}")] 
    ThreadAdd(String),  
    #[error("other error - {0}")]
    Other(String),
}