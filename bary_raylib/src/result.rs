#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound,
    BadPartName,
    BadBlueprint,
}

pub type BaryResult<E> = Result<E, BaryError>;
