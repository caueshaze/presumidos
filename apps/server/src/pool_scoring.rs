use crate::error::ServerFnError;

#[cfg(feature = "server")]
pub async fn football_scoring_config(
    token: String,
    pool_id: String,
) -> Result<crate::models::FootballScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let member: Option<(String,)> =
        sqlx::query_as("SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("football_config_membership", e))?;
    if member.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    sqlx::query_as("SELECT exact_score_points,correct_result_exact_side_points,correct_result_points,incorrect_result_points,knockout_bonus_points FROM football_pool_scoring WHERE pool_id=?1").bind(&pool_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("football_config_load",e))
}

#[cfg(feature = "server")]
pub async fn update_football_scoring_config(
    token: String,
    pool_id: String,
    config: crate::models::FootballScoringConfig,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if [
        config.exact_score_points,
        config.correct_result_exact_side_points,
        config.correct_result_points,
        config.incorrect_result_points,
        config.knockout_bonus_points,
    ]
    .iter()
    .any(|v| !(0..=1000).contains(v))
    {
        return Err(crate::security::public_error(
            "Pontuação deve estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT created_by FROM pools p WHERE p.id=?1 AND (p.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)) AND p.predictions_closed_at IS NULL AND p.closed_at IS NULL").bind(&pool_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("football_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Apenas o dono ou admin pode alterar antes do primeiro lock.",
        ));
    }
    sqlx::query("UPDATE football_pool_scoring SET exact_score_points=?2,correct_result_exact_side_points=?3,correct_result_points=?4,incorrect_result_points=?5,knockout_bonus_points=?6,updated_at=datetime('now') WHERE pool_id=?1").bind(&pool_id).bind(config.exact_score_points).bind(config.correct_result_exact_side_points).bind(config.correct_result_points).bind(config.incorrect_result_points).bind(config.knockout_bonus_points).execute(db).await.map_err(|e|crate::security::internal_error("football_config_update",e))?;
    crate::scoring::recalculate_all_breakdowns(Some(&session.user_id)).await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn custom_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::CustomItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let ok: Option<(String,)> = sqlx::query_as(
        "SELECT ?2 WHERE EXISTS (SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?2) OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)",
    )
    .bind(&pool_id)
    .bind(&session.user_id)
    .fetch_optional(db)
    .await
    .map_err(|e| crate::security::internal_error("custom_config_member", e))?;
    if ok.is_none() {
        return Err(crate::security::public_error(
            "Voce nao participa deste bolao.",
        ));
    }
    sqlx::query_as("SELECT pool_id,item_id,correct_points,incorrect_points FROM custom_pool_item_scoring WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("custom_config_load",e))
}

#[cfg(feature = "server")]
pub async fn update_custom_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    correct: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if !(0..=1000).contains(&correct) || !(0..=1000).contains(&incorrect) {
        return Err(crate::security::public_error(
            "Pontuação deve estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT p.created_by FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id WHERE p.id=?1 AND (p.created_by=?2 OR EXISTS (SELECT 1 FROM users WHERE id=?2 AND is_admin=1)) AND p.predictions_closed_at IS NULL AND p.closed_at IS NULL AND pi.id=?3 AND pi.kind='single_choice'").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("custom_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Apenas o dono ou admin pode alterar antes do lock.",
        ));
    }
    sqlx::query("UPDATE custom_pool_item_scoring SET correct_points=?3,incorrect_points=?4,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(correct).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("custom_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn numeric_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::NumericItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    let row:Option<(String,String,i64,i64,i64,i64,i64)>=sqlx::query_as("SELECT s.pool_id,s.item_id,s.exact_points,s.tolerance_scaled,s.within_tolerance_points,s.incorrect_points,n.decimal_places FROM numeric_pool_item_scoring s JOIN numeric_questions n ON n.item_id=s.item_id WHERE s.pool_id=?1 AND s.item_id=?2 AND (EXISTS(SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?3) OR EXISTS(SELECT 1 FROM users WHERE id=?3 AND is_admin=1))").bind(&pool_id).bind(&item_id).bind(&session.user_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("numeric_config_load",e))?;
    row.map(
        |(pool_id, item_id, exact, tolerance, within, incorrect, places)| {
            crate::models::NumericItemScoringConfig {
                pool_id,
                item_id,
                exact_points: exact,
                tolerance: crate::numeric::display_scaled(tolerance, places as u8),
                within_tolerance_points: within,
                incorrect_points: incorrect,
            }
        },
    )
    .ok_or_else(|| crate::security::public_error("Configuração numeric inválida."))
}

#[cfg(feature = "server")]
pub async fn update_numeric_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    exact: i64,
    tolerance: String,
    within: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if !(0..=1000).contains(&exact)
        || !(0..=1000).contains(&within)
        || !(0..=1000).contains(&incorrect)
    {
        return Err(crate::security::public_error(
            "Pontos devem estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let places:Option<(i64,)>=sqlx::query_as("SELECT n.decimal_places FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id JOIN numeric_questions n ON n.item_id=pi.id LEFT JOIN users u ON u.id=?2 WHERE p.id=?1 AND pi.id=?3 AND (p.created_by=?2 OR u.is_admin=1) AND p.predictions_closed_at IS NULL AND p.closed_at IS NULL").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("numeric_config_owner",e))?;
    let Some((places,)) = places else {
        return Err(crate::security::public_error(
            "Somente o dono do bolão pode alterar regras antes do lock.",
        ));
    };
    let tolerance = crate::numeric::parse_scaled(&tolerance, places as u8)
        .map_err(crate::security::public_error)?;
    if tolerance < 0 {
        return Err(crate::security::public_error(
            "Tolerância não pode ser negativa.",
        ));
    }
    sqlx::query("UPDATE numeric_pool_item_scoring SET exact_points=?3,tolerance_scaled=?4,within_tolerance_points=?5,incorrect_points=?6,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(exact).bind(tolerance).bind(within).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("numeric_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await
}

#[cfg(feature = "server")]
pub async fn multiple_choice_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
) -> Result<crate::models::MultipleChoiceItemScoringConfig, ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    let db = crate::db::pool();
    sqlx::query_as("SELECT s.pool_id,s.item_id,s.exact_points,s.partial_points,s.incorrect_points FROM multiple_choice_pool_item_scoring s WHERE s.pool_id=?1 AND s.item_id=?2 AND (EXISTS(SELECT 1 FROM pool_members WHERE pool_id=?1 AND user_id=?3) OR EXISTS(SELECT 1 FROM users WHERE id=?3 AND is_admin=1))").bind(&pool_id).bind(&item_id).bind(&session.user_id).fetch_one(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_load",e))
}
#[cfg(feature = "server")]
pub async fn update_multiple_choice_item_scoring_config(
    token: String,
    pool_id: String,
    item_id: String,
    exact: i64,
    partial: i64,
    incorrect: i64,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    if [exact, partial, incorrect]
        .iter()
        .any(|v| !(0..=1000).contains(v))
    {
        return Err(crate::security::public_error(
            "Pontos devem estar entre 0 e 1000.",
        ));
    }
    let db = crate::db::pool();
    let owner:Option<(String,)>=sqlx::query_as("SELECT p.created_by FROM pools p JOIN prediction_items pi ON pi.event_version_id=p.event_version_id LEFT JOIN users u ON u.id=?2 WHERE p.id=?1 AND pi.id=?3 AND pi.kind='multiple_choice' AND (p.created_by=?2 OR u.is_admin=1) AND p.predictions_closed_at IS NULL AND p.closed_at IS NULL").bind(&pool_id).bind(&session.user_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_owner",e))?;
    if owner.is_none() {
        return Err(crate::security::public_error(
            "Somente o dono do bolão pode alterar regras antes do lock.",
        ));
    }
    sqlx::query("UPDATE multiple_choice_pool_item_scoring SET exact_points=?3,partial_points=?4,incorrect_points=?5,updated_at=datetime('now') WHERE pool_id=?1 AND item_id=?2").bind(&pool_id).bind(&item_id).bind(exact).bind(partial).bind(incorrect).execute(db).await.map_err(|e|crate::security::internal_error("multiple_choice_config_update",e))?;
    crate::scoring::recalculate_custom_breakdowns().await
}
