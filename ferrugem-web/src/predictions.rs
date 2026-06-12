use dioxus::prelude::*;

use crate::auth;
use crate::auth::AuthState;
use crate::matches::{
    get_my_predictions, is_knockout_released, list_matches, set_knockout_released, set_match_result,
    submit_prediction, update_match_teams,
};
use crate::models::{KnockoutEntry, MatchRecord, PredictionRecord};
use crate::AuthPendingCard;
use crate::Route;
use chrono::{DateTime, Utc};

fn format_kickoff(kickoff: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(kickoff)
        .map(|dt| dt.format("%d/%m %H:%M").to_string())
        .unwrap_or_else(|_| kickoff.to_string())
}

/// Lado vencedor de um placar de tempo normal, ou `None` em empate.
fn winner_side(home: i64, away: i64) -> Option<&'static str> {
    match home.cmp(&away) {
        std::cmp::Ordering::Greater => Some("home"),
        std::cmp::Ordering::Less => Some("away"),
        std::cmp::Ordering::Equal => None,
    }
}

fn parse_kickoff_utc(kickoff: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(kickoff)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn is_match_locked_at(kickoff: &str, now: DateTime<Utc>) -> bool {
    parse_kickoff_utc(kickoff).is_some_and(|kickoff_at| kickoff_at <= now)
}

fn requires_admin_reauth(message: &str) -> bool {
    message.contains("SECURITY:ADMIN_REAUTH_REQUIRED")
}

#[cfg(target_arch = "wasm32")]
async fn prompt_admin_password() -> Option<String> {
    web_sys::window()
        .and_then(|window| {
            window
                .prompt_with_message("Confirme sua senha de administrador para continuar")
                .ok()
        })
        .flatten()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
async fn prompt_admin_password() -> Option<String> {
    None
}

async fn resolve_admin_reauth(csrf_token: String) -> Result<bool, ServerFnError> {
    let Some(password) = prompt_admin_password().await else {
        return Ok(false);
    };

    auth::confirm_admin_password(password, csrf_token).await?;
    Ok(true)
}

#[cfg(target_arch = "wasm32")]
fn current_client_utc() -> Option<DateTime<Utc>> {
    let millis = js_sys::Date::now() as i64;
    DateTime::<Utc>::from_timestamp_millis(millis)
}

#[cfg(not(target_arch = "wasm32"))]
fn current_client_utc() -> Option<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp_millis(Utc::now().timestamp_millis())
}

#[component]
pub fn Predictions() -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let navigator = use_navigator();

    use_effect(move || {
        let state = auth();
        if !state.loading && state.user.is_none() {
            navigator.push(Route::Login {});
        }
    });

    let auth_state = auth();
    if auth_state.loading {
        return rsx! { AuthPendingCard { message: "Preparando seus palpites...".to_string() } };
    }

    if auth_state.user.is_none() {
        return rsx! { AuthPendingCard { message: "Redirecionando para o login...".to_string() } };
    }

    let mut now_utc = use_signal(|| None::<DateTime<Utc>>);
    use_effect(move || {
        now_utc.set(current_client_utc());
    });

    let mut data = use_resource(move || {
        let token = auth().token.clone();
        async move {
            let matches = list_matches(token.clone()).await?;
            let predictions = get_my_predictions(token).await?;
            let knockout_released = is_knockout_released().await?;
            Ok::<_, ServerFnError>((matches, predictions, knockout_released))
        }
    });

    rsx! {
        div {
            class: "page",
            h1 { "Palpites" }
            p { "Dê seu palpite de placar para cada partida antes do apito inicial." }
            p { class: "rules-hint",
                "Em jogos de mata-mata, além do placar no tempo normal, escolha quem se classifica. "
                "Se apostar em empate no tempo normal, você pode marcar que o jogo vai para os pênaltis "
                "e, opcionalmente, palpitar o placar da disputa."
            }

            {
                match data.value()() {
                    Some(Ok((matches, predictions, knockout_released))) => {
                        let now = now_utc();
                        let is_admin = auth().user.map(|u| u.is_admin).unwrap_or(false);
                        let token = auth().token.clone();

                        let items: Vec<_> = matches
                            .into_iter()
                            .map(|game| {
                                let prediction = predictions
                                    .iter()
                                    .find(|p| p.match_id == game.id)
                                    .cloned();
                                let locked = now.is_some_and(|current| is_match_locked_at(&game.kickoff, current));
                                (game, prediction, locked)
                            })
                            .collect();

                        rsx! {
                            if is_admin {
                                KnockoutControl {
                                    token: token.clone(),
                                    released: knockout_released,
                                    on_changed: move |_| data.restart(),
                                }
                            }
                            div {
                                id: "matches-list",
                                for (game, prediction, locked) in items {
                                    MatchCard {
                                        key: "{game.id}",
                                        game,
                                        prediction,
                                        token: token.clone(),
                                        locked,
                                        is_admin,
                                        on_changed: move |_| data.restart(),
                                    }
                                }
                            }
                        }
                    }
                    Some(Err(e)) => rsx! {
                        div { class: "error-banner", "Erro ao carregar partidas: {e}" }
                    },
                    None => rsx! {
                        div { class: "card", p { "Carregando..." } }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_match_locked_at, parse_kickoff_utc};
    use chrono::{TimeZone, Utc};

    #[test]
    fn parses_rfc3339_kickoff_as_utc() {
        let kickoff = parse_kickoff_utc("2026-06-12T18:00:00Z").expect("kickoff should parse");
        assert_eq!(kickoff, Utc.with_ymd_and_hms(2026, 6, 12, 18, 0, 0).unwrap());
    }

    #[test]
    fn match_lock_respects_instant_not_string_shape() {
        let before = Utc.with_ymd_and_hms(2026, 6, 12, 17, 59, 59).unwrap();
        let at = Utc.with_ymd_and_hms(2026, 6, 12, 18, 0, 0).unwrap();
        let after = Utc.with_ymd_and_hms(2026, 6, 12, 18, 0, 1).unwrap();
        let kickoff = "2026-06-12T18:00:00Z";

        assert!(!is_match_locked_at(kickoff, before));
        assert!(is_match_locked_at(kickoff, at));
        assert!(is_match_locked_at(kickoff, after));
    }
}

#[component]
fn KnockoutControl(token: String, released: bool, on_changed: EventHandler<()>) -> Element {
    let auth = use_context::<Signal<AuthState>>();
    let mut saving = use_signal(|| false);
    let mut error = use_signal(String::new);

    let toggle = move |_| {
        let token = token.clone();
        let csrf_token = auth().csrf_token.clone();
        async move {
            saving.set(true);
            error.set(String::new());
            match set_knockout_released(token.clone(), !released, csrf_token.clone()).await {
                Ok(_) => on_changed.call(()),
                Err(ServerFnError::ServerError { message, .. }) => {
                    if requires_admin_reauth(&message) {
                        match resolve_admin_reauth(csrf_token.clone()).await {
                            Ok(true) => match set_knockout_released(token, !released, csrf_token).await {
                                Ok(_) => on_changed.call(()),
                                Err(ServerFnError::ServerError { message, .. }) => error.set(message),
                                Err(e) => error.set(e.to_string()),
                            },
                            Ok(false) => error.set("Confirmação de administrador cancelada.".to_string()),
                            Err(ServerFnError::ServerError { message, .. }) => error.set(message),
                            Err(e) => error.set(e.to_string()),
                        }
                    } else {
                        error.set(message)
                    }
                }
                Err(e) => error.set(e.to_string()),
            }
            saving.set(false);
        }
    };

    rsx! {
        div {
            class: "card admin-phase-control",
            h3 { "Fases do mata-mata (admin)" }
            if released {
                p { "O mata-mata está liberado e visível para todos os participantes." }
            } else {
                p { "O mata-mata está oculto. Você ainda vê todos os jogos para montar os confrontos; libere quando a fase de grupos terminar." }
            }
            if !error().is_empty() {
                div { class: "error-banner", "{error()}" }
            }
            button {
                class: if released { "btn btn-outline" } else { "btn btn-primary" },
                disabled: saving(),
                onclick: toggle,
                if saving() {
                    "Salvando..."
                } else if released {
                    "Ocultar mata-mata"
                } else {
                    "Liberar mata-mata"
                }
            }
        }
    }
}

#[component]
fn MatchCard(
    game: MatchRecord,
    prediction: Option<PredictionRecord>,
    token: String,
    locked: bool,
    is_admin: bool,
    on_changed: EventHandler<()>,
) -> Element {
    let knockout = crate::models::is_knockout(game.phase.as_deref());

    let initial_home = prediction.as_ref().map(|p| p.home_score).unwrap_or(0);
    let initial_away = prediction.as_ref().map(|p| p.away_score).unwrap_or(0);

    let mut home_guess = use_signal(|| initial_home);
    let mut away_guess = use_signal(|| initial_away);
    let mut saved_message = use_signal(String::new);
    let mut error_message = use_signal(String::new);
    let mut is_saving = use_signal(|| false);

    // Campos de mata-mata do palpite.
    let initial_qualifier = prediction
        .as_ref()
        .and_then(|p| p.qualifier.clone())
        .or_else(|| winner_side(initial_home, initial_away).map(String::from))
        .unwrap_or_else(|| "home".to_string());
    let mut qualifier = use_signal(|| initial_qualifier);
    let mut qualifier_touched =
        use_signal(|| prediction.as_ref().and_then(|p| p.qualifier.as_ref()).is_some());
    let mut went_pens =
        use_signal(|| prediction.as_ref().map(|p| p.went_to_penalties).unwrap_or(false));
    let mut pen_home = use_signal(|| prediction.as_ref().and_then(|p| p.penalty_home_score));
    let mut pen_away = use_signal(|| prediction.as_ref().and_then(|p| p.penalty_away_score));

    let mut result_home = use_signal(|| game.home_score);
    let mut result_away = use_signal(|| game.away_score);
    let mut result_error = use_signal(String::new);
    let mut result_saving = use_signal(|| false);

    // Campos de mata-mata do resultado oficial (admin).
    let initial_result_qualifier = game
        .qualifier
        .clone()
        .or_else(|| {
            winner_side(game.home_score.unwrap_or(0), game.away_score.unwrap_or(0)).map(String::from)
        })
        .unwrap_or_else(|| "home".to_string());
    let mut result_qualifier = use_signal(|| initial_result_qualifier);
    let mut result_qual_touched = use_signal(|| game.qualifier.is_some());
    let mut result_pens = use_signal(|| game.went_to_penalties);
    let mut result_pen_home = use_signal(|| game.penalty_home_score);
    let mut result_pen_away = use_signal(|| game.penalty_away_score);

    let mut team_home = use_signal(|| game.home_team.clone());
    let mut team_away = use_signal(|| game.away_team.clone());
    let mut teams_error = use_signal(String::new);
    let mut teams_saving = use_signal(|| false);

    // Resumo do resultado oficial (texto pronto, para o rsx).
    let official_qualifier_suffix = if knockout {
        game.qualifier
            .as_deref()
            .map(|q| {
                let team = if q == "home" { &game.home_team } else { &game.away_team };
                format!(" — {team} classificou")
            })
            .unwrap_or_default()
    } else {
        String::new()
    };
    let penalty_result_label = match (game.penalty_home_score, game.penalty_away_score) {
        (Some(ph), Some(pa)) => format!("Pênaltis: {ph} x {pa}"),
        _ => "Decidido nos pênaltis".to_string(),
    };

    let match_id = game.id.clone();
    let token_for_save = token.clone();
    let auth = use_context::<Signal<AuthState>>();
    let save = move |evt: FormEvent| {
        evt.prevent_default();

        let match_id = match_id.clone();
        let token = token_for_save.clone();
        let csrf_token = auth().csrf_token.clone();
        async move {
            is_saving.set(true);
            saved_message.set(String::new());
            error_message.set(String::new());

            let knockout_entry = if knockout {
                let pens = went_pens() && home_guess() == away_guess();
                KnockoutEntry {
                    qualifier: Some(qualifier()),
                    went_to_penalties: pens,
                    penalty_home: if pens { pen_home() } else { None },
                    penalty_away: if pens { pen_away() } else { None },
                }
            } else {
                KnockoutEntry::default()
            };

            match submit_prediction(
                token,
                match_id,
                home_guess(),
                away_guess(),
                knockout_entry,
                csrf_token,
            )
            .await
            {
                Ok(_) => saved_message.set("Palpite salvo!".to_string()),
                Err(ServerFnError::ServerError { message, .. }) => error_message.set(message),
                Err(e) => error_message.set(e.to_string()),
            }

            is_saving.set(false);
        }
    };

    let match_id = game.id.clone();
    let token_for_result = token.clone();
    let save_result = move |evt: FormEvent| {
        evt.prevent_default();

        let match_id = match_id.clone();
        let token = token_for_result.clone();
        let csrf_token = auth().csrf_token.clone();
        async move {
            result_saving.set(true);
            result_error.set(String::new());

            let home = result_home().unwrap_or(0);
            let away = result_away().unwrap_or(0);

            let knockout_entry = if knockout {
                let pens = result_pens() && home == away;
                KnockoutEntry {
                    qualifier: Some(result_qualifier()),
                    went_to_penalties: pens,
                    penalty_home: if pens { result_pen_home() } else { None },
                    penalty_away: if pens { result_pen_away() } else { None },
                }
            } else {
                KnockoutEntry::default()
            };

            match set_match_result(
                token.clone(),
                match_id.clone(),
                home,
                away,
                knockout_entry.clone(),
                csrf_token.clone(),
            )
            .await
            {
                Ok(_) => on_changed.call(()),
                Err(ServerFnError::ServerError { message, .. }) => {
                    if requires_admin_reauth(&message) {
                        match resolve_admin_reauth(csrf_token.clone()).await {
                            Ok(true) => match set_match_result(
                                token,
                                match_id,
                                home,
                                away,
                                knockout_entry,
                                csrf_token,
                            )
                            .await
                            {
                                Ok(_) => on_changed.call(()),
                                Err(ServerFnError::ServerError { message, .. }) => result_error.set(message),
                                Err(e) => result_error.set(e.to_string()),
                            },
                            Ok(false) => result_error.set("Confirmação de administrador cancelada.".to_string()),
                            Err(ServerFnError::ServerError { message, .. }) => result_error.set(message),
                            Err(e) => result_error.set(e.to_string()),
                        }
                    } else {
                        result_error.set(message)
                    }
                }
                Err(e) => result_error.set(e.to_string()),
            }

            result_saving.set(false);
        }
    };

    let match_id = game.id.clone();
    let save_teams = move |evt: FormEvent| {
        evt.prevent_default();

        let match_id = match_id.clone();
        let token = token.clone();
        let csrf_token = auth().csrf_token.clone();
        async move {
            teams_saving.set(true);
            teams_error.set(String::new());

            match update_match_teams(
                token.clone(),
                match_id.clone(),
                team_home(),
                team_away(),
                csrf_token.clone(),
            )
            .await
            {
                Ok(_) => on_changed.call(()),
                Err(ServerFnError::ServerError { message, .. }) => {
                    if requires_admin_reauth(&message) {
                        match resolve_admin_reauth(csrf_token.clone()).await {
                            Ok(true) => match update_match_teams(
                                token,
                                match_id,
                                team_home(),
                                team_away(),
                                csrf_token,
                            )
                            .await
                            {
                                Ok(_) => on_changed.call(()),
                                Err(ServerFnError::ServerError { message, .. }) => teams_error.set(message),
                                Err(e) => teams_error.set(e.to_string()),
                            },
                            Ok(false) => teams_error.set("Confirmação de administrador cancelada.".to_string()),
                            Err(ServerFnError::ServerError { message, .. }) => teams_error.set(message),
                            Err(e) => teams_error.set(e.to_string()),
                        }
                    } else {
                        teams_error.set(message)
                    }
                }
                Err(e) => teams_error.set(e.to_string()),
            }

            teams_saving.set(false);
        }
    };

    rsx! {
        div {
            class: "card match-card",
            div {
                class: "match-header",
                div {
                    class: "match-teams",
                    "{game.home_team} vs {game.away_team}"
                }
                div {
                    class: "match-badges",
                    if let Some(phase) = &game.phase {
                        span { class: "badge-phase", "{phase}" }
                    }
                    if let Some(group) = &game.group_name {
                        span { class: "badge-group", "Grupo {group}" }
                    }
                }
            }
            div { class: "match-date", "{format_kickoff(&game.kickoff)}" }

            if let (Some(hs), Some(as_)) = (game.home_score, game.away_score) {
                p { "Resultado oficial: {hs} x {as_}{official_qualifier_suffix}" }
                if knockout && game.went_to_penalties {
                    p { class: "match-penalty-result", "{penalty_result_label}" }
                }
            }

            if locked {
                p { class: "match-locked", "Partida já iniciada — palpites encerrados." }
            } else {
                form {
                    onsubmit: save,
                    if knockout {
                        label { class: "field-label", "Placar no tempo normal" }
                    }
                    div {
                        class: "score-inputs",
                        input {
                            r#type: "number",
                            min: "0",
                            value: "{home_guess()}",
                            oninput: move |e| {
                                let v = e.value().parse().unwrap_or(0);
                                home_guess.set(v);
                                if knockout {
                                    if !qualifier_touched() {
                                        if let Some(w) = winner_side(v, away_guess()) {
                                            qualifier.set(w.to_string());
                                        }
                                    }
                                    if v != away_guess() {
                                        went_pens.set(false);
                                    }
                                }
                            },
                        }
                        span { class: "vs", "x" }
                        input {
                            r#type: "number",
                            min: "0",
                            value: "{away_guess()}",
                            oninput: move |e| {
                                let v = e.value().parse().unwrap_or(0);
                                away_guess.set(v);
                                if knockout {
                                    if !qualifier_touched() {
                                        if let Some(w) = winner_side(home_guess(), v) {
                                            qualifier.set(w.to_string());
                                        }
                                    }
                                    if home_guess() != v {
                                        went_pens.set(false);
                                    }
                                }
                            },
                        }
                    }
                    if knockout {
                        div {
                            class: "knockout-fields",
                            label { class: "field-label", "Quem se classifica" }
                            select {
                                value: "{qualifier()}",
                                onchange: move |e| {
                                    qualifier.set(e.value());
                                    qualifier_touched.set(true);
                                },
                                option { value: "home", "{game.home_team}" }
                                option { value: "away", "{game.away_team}" }
                            }
                            if home_guess() == away_guess() {
                                label {
                                    class: "checkbox-line",
                                    input {
                                        r#type: "checkbox",
                                        checked: went_pens(),
                                        onchange: move |e| went_pens.set(e.checked()),
                                    }
                                    "Foi para os pênaltis?"
                                }
                                if went_pens() {
                                    label { class: "field-label", "Placar dos pênaltis (opcional)" }
                                    div {
                                        class: "score-inputs",
                                        input {
                                            r#type: "number",
                                            min: "0",
                                            value: "{pen_home().unwrap_or(0)}",
                                            oninput: move |e| pen_home.set(e.value().parse().ok()),
                                        }
                                        span { class: "vs", "x" }
                                        input {
                                            r#type: "number",
                                            min: "0",
                                            value: "{pen_away().unwrap_or(0)}",
                                            oninput: move |e| pen_away.set(e.value().parse().ok()),
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !error_message().is_empty() {
                        div { class: "error-banner", "{error_message()}" }
                    }
                    if !saved_message().is_empty() {
                        p { class: "match-saved", "{saved_message()}" }
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        disabled: is_saving(),
                        if is_saving() { "Salvando..." } else { "Salvar palpite" }
                    }
                }
            }

            if is_admin {
                form {
                    class: "admin-result-form",
                    onsubmit: save_teams,
                    h4 { "Admin: montar confronto" }
                    div {
                        class: "team-inputs",
                        input {
                            r#type: "text",
                            value: "{team_home()}",
                            oninput: move |e| team_home.set(e.value()),
                        }
                        span { class: "vs", "x" }
                        input {
                            r#type: "text",
                            value: "{team_away()}",
                            oninput: move |e| team_away.set(e.value()),
                        }
                    }
                    if !teams_error().is_empty() {
                        div { class: "error-banner", "{teams_error()}" }
                    }
                    button {
                        class: "btn btn-outline",
                        r#type: "submit",
                        disabled: teams_saving(),
                        if teams_saving() { "Salvando..." } else { "Salvar confronto" }
                    }
                }
                form {
                    class: "admin-result-form",
                    onsubmit: save_result,
                    h4 { "Admin: lançar resultado oficial" }
                    if knockout {
                        label { class: "field-label", "Resultado no tempo normal" }
                    }
                    div {
                        class: "score-inputs",
                        input {
                            r#type: "number",
                            min: "0",
                            value: "{result_home().unwrap_or(0)}",
                            oninput: move |e| {
                                let v: Option<i64> = e.value().parse().ok();
                                result_home.set(v);
                                if knockout {
                                    let h = v.unwrap_or(0);
                                    let a = result_away().unwrap_or(0);
                                    if !result_qual_touched() {
                                        if let Some(w) = winner_side(h, a) {
                                            result_qualifier.set(w.to_string());
                                        }
                                    }
                                    if h != a {
                                        result_pens.set(false);
                                    }
                                }
                            },
                        }
                        span { class: "vs", "x" }
                        input {
                            r#type: "number",
                            min: "0",
                            value: "{result_away().unwrap_or(0)}",
                            oninput: move |e| {
                                let v: Option<i64> = e.value().parse().ok();
                                result_away.set(v);
                                if knockout {
                                    let h = result_home().unwrap_or(0);
                                    let a = v.unwrap_or(0);
                                    if !result_qual_touched() {
                                        if let Some(w) = winner_side(h, a) {
                                            result_qualifier.set(w.to_string());
                                        }
                                    }
                                    if h != a {
                                        result_pens.set(false);
                                    }
                                }
                            },
                        }
                    }
                    if knockout {
                        div {
                            class: "knockout-fields",
                            label { class: "field-label", "Quem se classifica" }
                            select {
                                value: "{result_qualifier()}",
                                onchange: move |e| {
                                    result_qualifier.set(e.value());
                                    result_qual_touched.set(true);
                                },
                                option { value: "home", "{game.home_team}" }
                                option { value: "away", "{game.away_team}" }
                            }
                            if result_home().unwrap_or(0) == result_away().unwrap_or(0) {
                                label {
                                    class: "checkbox-line",
                                    input {
                                        r#type: "checkbox",
                                        checked: result_pens(),
                                        onchange: move |e| result_pens.set(e.checked()),
                                    }
                                    "Foi para os pênaltis?"
                                }
                                if result_pens() {
                                    label { class: "field-label", "Placar dos pênaltis (opcional)" }
                                    div {
                                        class: "score-inputs",
                                        input {
                                            r#type: "number",
                                            min: "0",
                                            value: "{result_pen_home().unwrap_or(0)}",
                                            oninput: move |e| result_pen_home.set(e.value().parse().ok()),
                                        }
                                        span { class: "vs", "x" }
                                        input {
                                            r#type: "number",
                                            min: "0",
                                            value: "{result_pen_away().unwrap_or(0)}",
                                            oninput: move |e| result_pen_away.set(e.value().parse().ok()),
                                        }
                                    }
                                }
                            }
                            p { class: "rules-hint",
                                "Empate no tempo normal sem pênaltis = classificado decidido na prorrogação (sem pontos extras)."
                            }
                        }
                    }
                    if !result_error().is_empty() {
                        div { class: "error-banner", "{result_error()}" }
                    }
                    button {
                        class: "btn btn-outline",
                        r#type: "submit",
                        disabled: result_saving(),
                        "Salvar resultado"
                    }
                }
            }
        }
    }
}
