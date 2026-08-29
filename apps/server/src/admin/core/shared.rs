#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct AdminMatchRow {
    pub(crate) id: String,
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) kickoff: String,
    pub(crate) group_name: Option<String>,
    pub(crate) phase: Option<String>,
    pub(crate) home_score: Option<i64>,
    pub(crate) away_score: Option<i64>,
    pub(crate) qualifier: Option<String>,
    pub(crate) went_to_penalties: bool,
    pub(crate) penalty_home_score: Option<i64>,
    pub(crate) penalty_away_score: Option<i64>,
    pub(crate) finished: bool,
    pub(crate) last_audit_at: Option<String>,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct AuditRow {
    pub(crate) id: String,
    pub(crate) actor_user_id: Option<String>,
    pub(crate) actor_username: Option<String>,
    pub(crate) action: String,
    pub(crate) target_type: String,
    pub(crate) target_id: Option<String>,
    pub(crate) ip_address: Option<String>,
    pub(crate) details_json: String,
    pub(crate) created_at: String,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(crate) struct PredictionAdminRow {
    pub(crate) user_id: String,
    pub(crate) username: String,
    pub(crate) pool_id: String,
    pub(crate) pool_name: String,
    pub(crate) match_id: String,
    pub(crate) home_team: String,
    pub(crate) away_team: String,
    pub(crate) kickoff: String,
    pub(crate) phase: Option<String>,
    pub(crate) home_score: Option<i64>,
    pub(crate) away_score: Option<i64>,
    pub(crate) qualifier: Option<String>,
    pub(crate) went_to_penalties: Option<bool>,
    pub(crate) penalty_home_score: Option<i64>,
    pub(crate) penalty_away_score: Option<i64>,
    pub(crate) override_id: Option<String>,
    pub(crate) override_reason: Option<String>,
    pub(crate) override_reopened_by: Option<String>,
    pub(crate) override_expires_at: Option<String>,
    pub(crate) override_used_at: Option<String>,
    pub(crate) override_created_at: Option<String>,
    pub(crate) override_revoked_at: Option<String>,
}

#[cfg(feature = "server")]
pub(crate) fn kickoff_matches_brasilia_date(kickoff: &str, date: &str) -> bool {
    if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return false;
    }
    let Some(brasilia_offset) = chrono::FixedOffset::west_opt(3 * 60 * 60) else {
        return false;
    };
    chrono::DateTime::parse_from_rfc3339(kickoff)
        .map(|dt| {
            dt.with_timezone(&brasilia_offset)
                .format("%Y-%m-%d")
                .to_string()
                == date
        })
        .unwrap_or(false)
}

#[cfg(feature = "server")]
pub(crate) fn to_match_record(row: AdminMatchRow) -> AdminMatchRecord {
    let admin_status = if row.finished {
        "finalized"
    } else {
        "scheduled"
    };

    AdminMatchRecord {
        match_record: MatchRecord {
            id: row.id,
            home_team: row.home_team,
            away_team: row.away_team,
            kickoff: row.kickoff,
            group_name: row.group_name,
            phase: row.phase,
            home_score: row.home_score,
            away_score: row.away_score,
            qualifier: row.qualifier,
            went_to_penalties: row.went_to_penalties,
            penalty_home_score: row.penalty_home_score,
            penalty_away_score: row.penalty_away_score,
            finished: row.finished,
        },
        admin_status: admin_status.to_string(),
        last_audit_at: row.last_audit_at,
    }
}
use crate::models::{AdminMatchRecord, MatchRecord};
