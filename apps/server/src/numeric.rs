use crate::error::ServerFnError;

pub const MAX_DECIMAL_PLACES: u8 = 6;

/// Parses a base-10 decimal string into the exact scaled SQLite representation.
/// Short fractional values are zero-padded; extra precision is rejected, never rounded.
pub fn parse_scaled(value: &str, decimal_places: u8) -> Result<i64, String> {
    if decimal_places > MAX_DECIMAL_PLACES {
        return Err("Casas decimais inválidas.".into());
    }
    let value = value.trim();
    if value.is_empty() {
        return Err("Valor numérico obrigatório.".into());
    }
    let (negative, value) = match value.strip_prefix('-') {
        Some(v) => (true, v),
        None => (false, value),
    };
    if value.is_empty() || value.starts_with('+') {
        return Err("Número decimal inválido.".into());
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap();
    let fraction = parts.next();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|c| c.is_ascii_digit())
        || fraction.is_some_and(|f| !f.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err("Número decimal inválido.".into());
    }
    let fraction = fraction.unwrap_or("");
    if fraction.len() > decimal_places as usize {
        return Err("Precisão decimal maior que a permitida.".into());
    }
    let scale = 10_i64
        .checked_pow(decimal_places as u32)
        .ok_or("Escala numérica inválida.")?;
    let integer = whole
        .parse::<i64>()
        .map_err(|_| "Número fora do limite.".to_string())?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<i64>()
            .map_err(|_| "Número fora do limite.".to_string())?
    };
    let padding = decimal_places as usize - fraction.len();
    let scaled = integer
        .checked_mul(scale)
        .and_then(|v| {
            fraction_value
                .checked_mul(10_i64.pow(padding as u32))
                .and_then(|f| v.checked_add(f))
        })
        .ok_or("Número fora do limite.")?;
    if negative {
        scaled.checked_neg().ok_or("Número fora do limite.".into())
    } else {
        Ok(scaled)
    }
}

pub fn display_scaled(value: i64, decimal_places: u8) -> String {
    if decimal_places == 0 {
        return value.to_string();
    }
    let scale = 10_i64.pow(decimal_places as u32);
    let sign = if value < 0 { "-" } else { "" };
    let abs = value.unsigned_abs();
    format!(
        "{sign}{}.{:0width$}",
        abs / scale as u64,
        abs % scale as u64,
        width = decimal_places as usize
    )
}

pub fn validate_question(
    decimal_places: i64,
    min: Option<i64>,
    max: Option<i64>,
) -> Result<u8, String> {
    let decimal_places = u8::try_from(decimal_places).map_err(|_| "Casas decimais inválidas.")?;
    if decimal_places > MAX_DECIMAL_PLACES {
        return Err("Casas decimais inválidas.".into());
    }
    if matches!((min,max),(Some(a),Some(b)) if a>b) {
        return Err("Valor mínimo deve ser menor ou igual ao máximo.".into());
    }
    Ok(decimal_places)
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericScoreOutcome {
    Exact,
    WithinTolerance,
    Incorrect,
}
#[cfg(test)]
impl NumericScoreOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::WithinTolerance => "within_tolerance",
            Self::Incorrect => "incorrect",
        }
    }
}
#[cfg(test)]
pub fn classify(
    predicted: i64,
    official: i64,
    tolerance: i64,
) -> Result<(NumericScoreOutcome, i64), String> {
    if tolerance < 0 {
        return Err("Tolerância inválida.".into());
    }
    let difference = predicted
        .checked_sub(official)
        .and_then(|v| v.checked_abs())
        .ok_or("Diferença numérica fora do limite.")?;
    Ok((
        if difference == 0 {
            NumericScoreOutcome::Exact
        } else if difference <= tolerance {
            NumericScoreOutcome::WithinTolerance
        } else {
            NumericScoreOutcome::Incorrect
        },
        difference,
    ))
}

#[cfg(feature = "server")]
pub async fn submit_prediction(
    token: String,
    pool_id: String,
    item_id: String,
    value: String,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let db = crate::db::pool();
    let row:Option<(String,Option<i64>,Option<i64>,i64,String)>=sqlx::query_as("SELECT pi.kind,n.min_value_scaled,n.max_value_scaled,n.decimal_places,pi.lock_at FROM pools p JOIN events e ON e.id=p.event_id JOIN prediction_items pi ON pi.event_version_id=p.event_version_id JOIN numeric_questions n ON n.item_id=pi.id WHERE p.id=?1 AND pi.id=?2 AND e.status='active'").bind(&pool_id).bind(&item_id).fetch_optional(db).await.map_err(|e|crate::security::internal_error("numeric_prediction_load",e))?;
    let Some((kind, min, max, places, lock_at)) = row else {
        return Err(crate::security::public_error("Bolão ou pergunta inválida."));
    };
    if kind != "numeric" {
        return Err(crate::security::public_error("Pergunta não é numérica."));
    }
    let places = validate_question(places, min, max).map_err(crate::security::public_error)?;
    let value = parse_scaled(&value, places).map_err(crate::security::public_error)?;
    if min.is_some_and(|v| value < v) || max.is_some_and(|v| value > v) {
        return Err(crate::security::public_error(
            "Valor fora dos limites da pergunta.",
        ));
    }
    let member: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM pool_members WHERE pool_id=?1 AND user_id=?2")
            .bind(&pool_id)
            .bind(&session.user_id)
            .fetch_optional(db)
            .await
            .map_err(|e| crate::security::internal_error("numeric_prediction_member", e))?;
    if member.is_none() {
        return Err(crate::security::public_error(
            "Você não participa deste bolão.",
        ));
    }
    if !crate::pool_access::can_write_predictions(&pool_id).await? {
        return Err(crate::security::public_error(
            "Os palpites deste bolão estão encerrados.",
        ));
    }
    if crate::prediction_access::can_edit_item("numeric", &lock_at, None, &session.user_id)
        .await?
        .is_none()
    {
        return Err(crate::security::public_error(
            "Esta pergunta está travada para palpite.",
        ));
    }
    let mut tx = db
        .begin()
        .await
        .map_err(|e| crate::security::internal_error("numeric_prediction_begin", e))?;
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO predictions(id,pool_id,user_id,item_id) VALUES(?1,?2,?3,?4) ON CONFLICT(pool_id,user_id,item_id) DO UPDATE SET submitted_at=datetime('now')").bind(id).bind(&pool_id).bind(&session.user_id).bind(&item_id).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_prediction_upsert",e))?;
    let prediction: (String,) =
        sqlx::query_as("SELECT id FROM predictions WHERE pool_id=?1 AND user_id=?2 AND item_id=?3")
            .bind(&pool_id)
            .bind(&session.user_id)
            .bind(&item_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| crate::security::internal_error("numeric_prediction_id", e))?;
    sqlx::query("INSERT INTO numeric_prediction_values(prediction_id,value_scaled) VALUES(?1,?2) ON CONFLICT(prediction_id) DO UPDATE SET value_scaled=excluded.value_scaled,updated_at=datetime('now')").bind(prediction.0).bind(value).execute(&mut *tx).await.map_err(|e|crate::security::internal_error("numeric_prediction_value",e))?;
    tx.commit()
        .await
        .map_err(|e| crate::security::internal_error("numeric_prediction_commit", e))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn set_result_authorized(
    token: String,
    item_id: String,
    value: String,
    pool_id: Option<String>,
    csrf: String,
) -> Result<(), ServerFnError> {
    let session = crate::auth::require_user(&token).await?;
    crate::security::require_csrf(&session.csrf_token, &csrf)?;
    let db = crate::db::pool();
    let row: Option<(i64, Option<i64>, Option<i64>, String)> =
        if let Some(pool_id) = pool_id.as_deref() {
            sqlx::query_as("SELECT n.decimal_places,n.min_value_scaled,n.max_value_scaled,pi.event_version_id FROM prediction_items pi JOIN numeric_questions n ON n.item_id=pi.id JOIN pools p ON p.event_version_id=pi.event_version_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND p.id=?3 AND (p.created_by=?2 OR u.is_admin=1)")
                .bind(&item_id)
                .bind(&session.user_id)
                .bind(pool_id)
                .fetch_optional(db)
                .await
        } else {
            sqlx::query_as("SELECT n.decimal_places,n.min_value_scaled,n.max_value_scaled,pi.event_version_id FROM prediction_items pi JOIN numeric_questions n ON n.item_id=pi.id JOIN events e ON e.id=pi.event_id LEFT JOIN users u ON u.id=?2 WHERE pi.id=?1 AND (e.created_by=?2 OR u.is_admin=1)")
                .bind(&item_id)
                .bind(&session.user_id)
                .fetch_optional(db)
                .await
        }
        .map_err(|e| crate::security::internal_error("numeric_result_authorization", e))?;
    let Some((places, min, max, version_id)) = row else {
        return Err(crate::security::public_error(
            "Somente o dono do bolão ou admin pode definir o resultado.",
        ));
    };
    let places = validate_question(places, min, max).map_err(crate::security::public_error)?;
    let scaled = parse_scaled(&value, places).map_err(crate::security::public_error)?;
    if min.is_some_and(|v| scaled < v) || max.is_some_and(|v| scaled > v) {
        return Err(crate::security::public_error(
            "Valor fora dos limites da pergunta.",
        ));
    }
    sqlx::query("UPDATE numeric_questions SET result_value_scaled=?2,updated_at=datetime('now') WHERE item_id=?1").bind(&item_id).bind(scaled).execute(db).await.map_err(|e|crate::security::internal_error("numeric_result_update",e))?;
    sqlx::query("INSERT INTO official_results(id,event_version_id,item_id,kind,state,value_scaled,updated_at) VALUES(?1,?2,?3,'numeric','resolved',?4,datetime('now')) ON CONFLICT(event_version_id,item_id) DO UPDATE SET state='resolved',option_id=NULL,option_ids_json=NULL,value_scaled=excluded.value_scaled,reason=NULL,updated_at=datetime('now')")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&version_id).bind(&item_id).bind(scaled)
        .execute(db).await.map_err(|e|crate::security::internal_error("numeric_result_official",e))?;
    crate::scoring::recalculate_custom_breakdowns().await?;
    crate::security::append_audit_log(
        db,
        Some(&session.user_id),
        "event_official_result_changed",
        "prediction_item",
        Some(&item_id),
        None,
        serde_json::json!({"value":display_scaled(scaled,places)}),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::{classify, display_scaled, parse_scaled, NumericScoreOutcome};
    #[test]
    fn decimal_strings_are_exact_and_never_rounded() {
        assert_eq!(parse_scaled("12.3", 2).unwrap(), 1230);
        assert_eq!(display_scaled(1230, 2), "12.30");
        assert!(parse_scaled("12.345", 2).is_err());
        assert!(parse_scaled("1e3", 2).is_err());
        assert!(parse_scaled("NaN", 2).is_err());
    }
    #[test]
    fn classification_includes_tolerance_boundary() {
        assert_eq!(
            classify(1000, 1000, 2).unwrap().0,
            NumericScoreOutcome::Exact
        );
        assert_eq!(
            classify(998, 1000, 2).unwrap().0,
            NumericScoreOutcome::WithinTolerance
        );
        assert_eq!(
            NumericScoreOutcome::WithinTolerance.as_str(),
            "within_tolerance"
        );
        assert_eq!(
            classify(1003, 1000, 2).unwrap().0,
            NumericScoreOutcome::Incorrect
        );
        assert_eq!(
            classify(999, 1000, 0).unwrap().0,
            NumericScoreOutcome::Incorrect
        );
    }
}
