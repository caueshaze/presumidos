use crate::error::ServerFnError;
use crate::models::MatchRecord;

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(super) struct MatchRow {
    pub(super) id: String,
    pub(super) home_team: String,
    pub(super) away_team: String,
    pub(super) kickoff: String,
    pub(super) group_name: Option<String>,
    pub(super) phase: Option<String>,
    pub(super) home_score: Option<i64>,
    pub(super) away_score: Option<i64>,
    pub(super) qualifier: Option<String>,
    pub(super) went_to_penalties: bool,
    pub(super) penalty_home_score: Option<i64>,
    pub(super) penalty_away_score: Option<i64>,
    pub(super) finished: bool,
}

#[cfg(feature = "server")]
#[derive(sqlx::FromRow)]
pub(super) struct MatchResultAuditRow {
    pub(super) phase: Option<String>,
    pub(super) home_score: Option<i64>,
    pub(super) away_score: Option<i64>,
    pub(super) qualifier: Option<String>,
    pub(super) went_to_penalties: bool,
    pub(super) penalty_home_score: Option<i64>,
    pub(super) penalty_away_score: Option<i64>,
}

#[cfg(feature = "server")]
pub(super) fn record_from_row(row: MatchRow) -> MatchRecord {
    MatchRecord {
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
    }
}

#[cfg(feature = "server")]
pub(super) async fn load_match_record(
    db: &sqlx::SqlitePool,
    match_id: &str,
) -> Result<MatchRecord, ServerFnError> {
    let row = sqlx::query_as::<_, MatchRow>(
        "SELECT id, home_team, away_team, kickoff, group_name, phase,
                home_score, away_score, qualifier, went_to_penalties,
                penalty_home_score, penalty_away_score, finished
         FROM matches WHERE id = ?1",
    )
    .bind(match_id)
    .fetch_one(db)
    .await
    .map_err(|error| crate::security::internal_error("load_match_record", error))?;
    Ok(record_from_row(row))
}
