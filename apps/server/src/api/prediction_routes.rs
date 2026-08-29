use super::*;
#[path = "prediction_routes/custom.rs"]
mod custom;
#[path = "prediction_routes/football.rs"]
mod football;
pub(crate) use custom::*;
pub(crate) use football::*;
