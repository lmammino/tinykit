use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeEventPayload {
    pub subscription_id: String,
    pub campaign_id: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeConfirmationTokenClaims {
    pub subscription_id: String,
    pub campaign_id: String,
    pub email: String,
    pub nbf: u64,
    pub iat: u64,
    pub exp: u64,
}

impl SubscribeConfirmationTokenClaims {
    pub fn new(
        subscription_id: String,
        campaign_id: String,
        email: String,
        expire_in_seconds: u64,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            subscription_id,
            campaign_id,
            email,
            nbf: now,
            iat: now,
            exp: now + expire_in_seconds,
        }
    }
}
