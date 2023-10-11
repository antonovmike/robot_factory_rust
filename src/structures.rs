use serde::{Deserialize, Serialize};
use validator::Validate;
use validator_derive::Validate;

use crate::db::validate_model_version;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Customer {
    #[validate(email)]
    email: String,
}

struct _Order {
    customer: Customer,
    robot_serial: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Robot {
    #[validate(length(min = 1, max = 5))]
    pub serial: String,
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
    pub created: String,
}