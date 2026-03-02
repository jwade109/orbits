#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound,
    BadPartName,
    BadBlueprint,
    FailedToSaveBlueprint,
    FailedToSaveGrid,
    IoError(String),
    SerdeYaml(String),
    Toml(toml::ser::Error),
}

pub type BaryResult<E> = Result<E, BaryError>;

impl From<std::io::Error> for BaryError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(format!("{:?}", value))
    }
}

impl From<serde_yaml::Error> for BaryError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::SerdeYaml(format!("{:?}", value))
    }
}

impl From<toml::ser::Error> for BaryError {
    fn from(value: toml::ser::Error) -> Self {
        Self::Toml(value)
    }
}
