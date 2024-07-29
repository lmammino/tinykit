use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeEventPayload {
    pub subscription_id: String,
    pub campaign_id: String,
    pub email: String,
}
