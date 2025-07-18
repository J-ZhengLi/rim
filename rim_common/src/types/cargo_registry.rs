use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct CargoRegistry {
    pub name: String,
    pub index: String,
}

impl<N: ToString, T: ToString> From<(N, T)> for CargoRegistry {
    fn from(value: (N, T)) -> Self {
        Self {
            name: value.0.to_string(),
            index: value.1.to_string(),
        }
    }
}
