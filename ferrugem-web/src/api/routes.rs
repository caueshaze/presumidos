use super::*;

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(health_live))
        .route("/health/ready", get(health_ready))
        .route("/internal/metrics", get(internal_metrics))
        .route("/contact", get(contact_info))
        .route("/settings/public", get(public_settings))
        .route("/auth/register", post(register))
        .route("/auth/register/confirm", post(register_confirm))
        .route("/auth/password-reset", post(password_reset))
        .route("/auth/password-reset/confirm", post(password_reset_confirm))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/current-user", get(current_user))
        .route("/auth/reauth", post(reauth))
        .route("/auth/username", post(change_username))
        .route("/auth/delete", post(delete_account))
        .route("/auth/csrf", get(csrf))
        .route("/notifications/status", get(notification_status))
        .route(
            "/notifications/preferences",
            post(update_notification_preference_handler),
        )
        .route(
            "/notifications/subscriptions",
            post(upsert_push_subscription_handler),
        )
        .route(
            "/notifications/subscriptions/remove",
            post(remove_push_subscription_handler),
        )
        .route("/pools", get(list_pools).post(create_pool))
        .route("/pools/dashboard", get(dashboard_pools))
        .route("/custom/events/mine", get(custom_events_mine))
        .route("/custom/events/available", get(custom_events_available))
        .route("/custom/events", post(custom_event_create))
        .route("/custom/events/{id}", get(custom_event_get))
        .route("/custom/events/{id}/draft", get(custom_event_draft))
        .route("/custom/events/{id}/update", post(custom_event_update))
        .route(
            "/custom/events/{id}/manifest",
            get(custom_event_manifest_export),
        )
        .route(
            "/custom/events/{id}/package",
            get(custom_event_package_export),
        )
        .route("/custom/events/{id}/cover", post(custom_event_cover_upload))
        .route(
            "/custom/events/{id}/cover/remove",
            post(custom_event_cover_remove),
        )
        .route("/custom/events/{id}/delete", post(custom_event_delete))
        .route("/custom/events/{id}/items", post(custom_event_add_item))
        .route(
            "/custom/events/{id}/items/numeric",
            post(custom_event_add_numeric_item),
        )
        .route(
            "/custom/events/{id}/items/multiple-choice",
            post(custom_event_add_multiple_choice_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/update",
            post(custom_event_update_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/delete",
            post(custom_event_delete_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/move",
            post(custom_event_move_item),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options",
            post(custom_event_add_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/update",
            post(custom_event_update_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/media",
            post(custom_event_update_option_media),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/image",
            post(custom_event_option_upload),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/image/remove",
            post(custom_event_option_remove),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/delete",
            post(custom_event_delete_option),
        )
        .route(
            "/custom/events/{id}/items/{item_id}/options/{option_id}/move",
            post(custom_event_move_option),
        )
        .route("/custom/events/{id}/publish", post(custom_event_publish))
        .route(
            "/public/pools/invite/{token}",
            get(public_pool_invite_preview),
        )
        .route("/pools/join", post(join_pool))
        .route("/pools/{pool_id}/leave", post(leave_pool))
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
        .route("/matches", get(list_matches))
        .route("/matches/knockout-released", get(knockout_released))
        .route("/predictions", get(my_predictions).post(submit_prediction))
        .route("/custom/questions", get(custom_questions))
        .route("/custom/event-showcase", get(custom_event_showcase))
        .route("/custom/media-progress", post(update_option_media_progress))
        .route(
            "/pools/{pool_id}/custom-member-predictions",
            get(custom_member_predictions),
        )
        .route("/custom/predictions", post(submit_single_choice_prediction))
        .route(
            "/custom/numeric-predictions",
            post(submit_numeric_prediction),
        )
        .route(
            "/custom/multiple-choice-predictions",
            post(submit_multiple_choice_prediction),
        )
        .route(
            "/admin/custom/questions/{item_id}/result",
            post(set_custom_question_result),
        )
        .route(
            "/admin/custom/questions/{item_id}/result-not-representable",
            post(mark_custom_result_not_representable),
        )
        .route(
            "/admin/custom/numeric/{item_id}/result",
            post(set_numeric_question_result),
        )
        .route(
            "/admin/custom/multiple-choice/{item_id}/result",
            post(set_multiple_choice_result),
        )
        .route(
            "/pools/{pool_id}/scoring/football",
            get(pool_football_scoring).post(update_pool_football_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/items/{item_id}",
            get(pool_custom_scoring).post(update_pool_custom_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/numeric/{item_id}",
            get(pool_numeric_scoring).post(update_pool_numeric_scoring),
        )
        .route(
            "/pools/{pool_id}/scoring/multiple-choice/{item_id}",
            get(pool_multiple_choice_scoring).post(update_pool_multiple_choice_scoring),
        )
        .route("/predictions/reopened", get(my_prediction_overrides))
        .route("/scoring/my-points", get(my_match_points))
        .route("/admin/overview", get(admin_overview))
        .route("/admin/events", get(admin_events))
        .route("/admin/events/{event_id}/delete", post(admin_event_delete))
        .route(
            "/admin/events/{event_id}/pool-creation",
            post(admin_event_availability),
        )
        .route(
            "/admin/events/{event_id}/versions/{version_id}/publish",
            post(admin_event_version_publish),
        )
        .route(
            "/admin/events/{event_id}/versions/{version_id}/restore",
            post(admin_event_version_restore),
        )
        .route(
            "/admin/events/{event_id}/manifest",
            get(admin_event_manifest_export),
        )
        .route(
            "/admin/events/{event_id}/package",
            get(admin_event_package_export),
        )
        .route("/admin/events/import/preview", post(admin_manifest_preview))
        .route("/admin/events/import/apply", post(admin_manifest_apply))
        .route(
            "/admin/events/import/package/preview",
            post(admin_package_preview),
        )
        .route(
            "/admin/events/import/package/apply",
            post(admin_package_apply),
        )
        .route("/admin/events/{event_id}/finish", post(admin_finish_event))
        .route("/admin/matches", get(admin_matches).post(create_match))
        .route("/admin/matches/{id}/audit", get(admin_match_audit))
        .route("/admin/matches/{id}/result", post(set_match_result))
        .route("/admin/matches/{id}/finished", post(set_match_finished))
        .route("/admin/matches/{id}/schedule", post(update_match_schedule))
        .route("/admin/matches/{id}/fixture", post(set_match_fixture))
        .route("/admin/fixtures/check", post(check_fixture))
        .route("/admin/matches/{id}/delete", post(delete_match))
        .route("/admin/knockout-released", post(set_knockout_released))
        .route("/admin/matches/{id}/teams", post(update_match_teams))
        .route("/admin/sync/status", get(admin_sync_status))
        .route("/admin/sync/run-now", post(admin_sync_run_now))
        .route("/admin/sync/backfill", post(admin_sync_backfill))
        .route("/admin/predictions", get(admin_predictions))
        .route("/admin/predictions/reopen", post(admin_prediction_reopen))
        .route(
            "/admin/predictions/reopen/revoke",
            post(admin_prediction_reopen_revoke),
        )
        .route(
            "/admin/scoring/recalculate-match",
            post(admin_recalculate_match),
        )
        .route(
            "/admin/scoring/recalculate-all",
            post(admin_recalculate_all),
        )
        .route(
            "/admin/scoring/users/{id}/breakdown",
            get(admin_user_breakdown),
        )
        .route("/admin/pools", get(admin_list_pools))
        .route("/admin/users", get(admin_list_users))
        .route("/admin/users/{id}/pools", get(admin_user_pools))
        .route("/admin/users/{id}/block", post(admin_block_user))
        .route("/admin/users/{id}/unblock", post(admin_unblock_user))
        .route(
            "/admin/users/{id}/invalidate-sessions",
            post(admin_invalidate_user_sessions),
        )
        .route(
            "/admin/users/{id}/password-reset",
            post(admin_trigger_user_password_reset),
        )
        .route("/admin/users/{id}/push", post(admin_send_push_to_user))
        .route("/admin/push/broadcast", post(admin_send_push_broadcast))
        .route(
            "/admin/pools/{pool_id}/members",
            get(admin_list_pool_members).post(admin_add_pool_member),
        )
        .route(
            "/admin/pools/{pool_id}/members/remove",
            post(admin_remove_pool_member),
        )
        .route("/admin/pool-reports", get(admin_list_pool_reports))
        .route(
            "/admin/pool-reports/{report_id}/status",
            post(admin_update_pool_report_status),
        )
        .route("/admin/audit", get(admin_audit))
        .route(
            "/admin/settings",
            get(admin_get_settings).post(admin_save_settings),
        )
        .route("/leaderboard", get(leaderboard))
        .fallback(api_not_found)
}
