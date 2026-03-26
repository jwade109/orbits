use bary_core::prelude::*;

#[derive(Debug, PartialEq, Eq)]
pub enum BaryError {
    EntityNotFound(Ent),
    BadPartName,
    BadBlueprint,
    FailedToSaveBlueprint,
    FailedToSaveGrid,
    SaveAlreadyExists,
    NoPrimaryComputer,
    NoPartsAt(PartCoord),
    NoPartsInLayer(PartLayer),
    IoError(String),
    SerdeYaml(String),
    TomlSer(toml::ser::Error),
    TomlDe(toml::de::Error),
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
        Self::TomlSer(value)
    }
}

impl From<toml::de::Error> for BaryError {
    fn from(value: toml::de::Error) -> Self {
        Self::TomlDe(value)
    }
}
