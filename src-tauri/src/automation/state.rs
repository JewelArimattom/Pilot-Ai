use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedState {
  pub kind: String,
  pub value: String
}

pub fn verify_expected(_expected: &[ExpectedState]) -> bool {
  true
}
