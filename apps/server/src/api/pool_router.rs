use super::*;

pub(super) fn pool_router() -> Router {
    Router::new()
        .route(
            "/public/pools/invite/{token}",
            get(public_pool_invite_preview),
        )
        .route("/pools/join", post(join_pool))
        .route("/pools/{pool_id}/leave", post(leave_pool))
        .route(
            "/pools/{pool_id}/close-predictions",
            post(close_predictions),
        )
        .route("/pools/{pool_id}/close", post(close_pool))
        .route("/pools/{pool_id}/reopen", post(reopen_pool))
        .route("/pools/{pool_id}/editorial", get(pool_editorial))
        .route(
            "/pools/{pool_id}/editorial/name",
            post(update_pool_editorial_name),
        )
        .route(
            "/pools/{pool_id}/editorial/options/{option_id}/links",
            post(replace_pool_option_links),
        )
        .route(
            "/pools/{pool_id}/editorial/options/{option_id}/links/reset",
            post(reset_pool_option_links),
        )
        .route(
            "/pools/{pool_id}/prediction-reuse",
            get(prediction_reuse_suggestion),
        )
        .route(
            "/pools/{pool_id}/prediction-reuse/copy",
            post(prediction_reuse_copy),
        )
        .route(
            "/pools/{pool_id}/prediction-reuse/start-empty",
            post(prediction_reuse_start_empty),
        )
        .route("/pools/{pool_id}/reports", post(create_pool_report))
        .route(
            "/pools/{pool_id}/member-predictions",
            get(pool_member_predictions),
        )
        .route(
            "/pools/{pool_id}/prediction-reactions",
            post(react_to_prediction),
        )
        .route(
            "/pools/{pool_id}/prediction-reactions/mark-seen",
            post(mark_prediction_reactions_seen),
        )
        .route("/pools/{pool_id}/breakdowns", get(pool_breakdowns))
        .route(
            "/pools/{pool_id}/adjustments",
            get(list_pool_adjustments).post(add_point_adjustment),
        )
        .route(
            "/pools/{pool_id}/adjustments/remove",
            post(remove_point_adjustment),
        )
        .route("/pools/{pool_id}/delete", post(delete_pool))
}
