use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageVotes {
    pub image_id: i64,
    pub rating: i64,
    pub curr_user_upvote: Option<bool>,
}
