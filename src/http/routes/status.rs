use axum::{extract::State, http::StatusCode, response::{IntoResponse, Response}};
use serde::Serialize;

use crate::state::appstate::AppState;

#[derive(Serialize)]
pub struct Statistics {
  total_packet_count: u32
}

impl IntoResponse for Statistics {
  fn into_response(self) -> Response {
    let s = match serde_json::to_string(&self) {
      Ok(s) => s,
      Err(_e) => {
        return (StatusCode::INTERNAL_SERVER_ERROR, _e.to_string()).into_response()
      }
    };

    (StatusCode::OK, s).into_response()
  }
}

pub async fn process(State(state): State<AppState>) -> Statistics {
  let total = state.interfaces.lock().unwrap().iter().fold(0, |acc, item| acc + item.count());
  Statistics{
    total_packet_count: total,
  }
}