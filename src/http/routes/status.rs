use axum::extract::State;
use serde::Serialize;

use crate::appstate::AppState;

#[derive(Serialize)]
struct Statistics {
  total_packet_count: u64
}

pub async fn process(State(_state): State<AppState<'static, ()>>) -> &'static str {
  "NOTOK"
}