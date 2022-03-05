use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct UserId(pub i32);
