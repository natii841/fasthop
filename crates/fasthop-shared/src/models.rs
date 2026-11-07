use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "sub_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SubStatus {
    Trial,
    Active,
    Paused,
    Expired,
}

impl SubStatus {
    pub const fn grants_access(self) -> bool {
        matches!(self, Self::Trial | Self::Active)
    }
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Trial,
            1 => Self::Active,
            2 => Self::Paused,
            _ => Self::Expired,
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Subscription {
    pub telegram_id: i64,
    pub token: String,
    pub status: SubStatus,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}
