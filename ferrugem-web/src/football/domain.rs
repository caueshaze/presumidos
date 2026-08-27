use serde::Deserialize;

// ---------------------------------------------------------------------------
// Estruturas da resposta do scoreboard externo (apenas o que usamos).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Event {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) competitions: Vec<Competition>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Competition {
    pub(crate) status: Status,
    #[serde(default)]
    pub(crate) competitors: Vec<Competitor>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Status {
    #[serde(default, rename = "displayClock")]
    pub(crate) display_clock: String,
    #[serde(rename = "type")]
    pub(crate) type_: StatusType,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct StatusType {
    /// "pre" | "in" | "post".
    #[serde(default)]
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) completed: bool,
    /// Ex.: "STATUS_FULL_TIME", "STATUS_FIRST_HALF".
    #[serde(default)]
    pub(crate) name: String,
    #[serde(default, rename = "shortDetail")]
    pub(crate) short_detail: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Competitor {
    #[serde(default, rename = "homeAway")]
    pub(crate) home_away: String,
    #[serde(default)]
    pub(crate) score: String,
    /// A fonte marca o classificado/vencedor do confronto.
    #[serde(default)]
    pub(crate) winner: bool,
    pub(crate) team: Team,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Team {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default, rename = "displayName")]
    pub(crate) display_name: String,
}

// --- Estruturas do endpoint `summary` (apenas a disputa de pênaltis) ---------

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Summary {
    #[serde(default)]
    pub(crate) shootout: Vec<ShootoutTeam>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub(crate) struct ShootoutTeam {
    /// Id da seleção na fonte (ex.: "202"); casa com `competitor.team.id`.
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) team: String,
    #[serde(default)]
    pub(crate) shots: Vec<Shot>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub(crate) struct Shot {
    #[serde(default, rename = "didScore")]
    pub(crate) did_score: bool,
}

pub(crate) fn parse_score(raw: &str) -> i64 {
    raw.trim().parse::<i64>().unwrap_or(0)
}

impl Event {
    pub(crate) fn competition(&self) -> Option<&Competition> {
        self.competitions.first()
    }
}

impl Competition {
    pub(crate) fn side(&self, which: &str) -> Option<&Competitor> {
        self.competitors.iter().find(|c| c.home_away == which)
    }
}

// ---------------------------------------------------------------------------
// Classificação de um evento externo: o que fazer com ele no banco.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GameApply {
    /// Em andamento — atualizar apenas o placar ao vivo.
    Live {
        home: i64,
        away: i64,
        status: String,
        elapsed: Option<i64>,
    },
    /// Fase de grupos encerrada — gravar o resultado oficial.
    FinalGroup {
        home: i64,
        away: i64,
        raw_status: String,
    },
    /// Mata-mata encerrado. O poller calcula o recorte completo (placar +
    /// classificado + pênaltis via `summary`) e a etapa de aplicação decide entre
    /// autofinalizar com segurança ou deixar pendente para revisão do admin.
    KnockoutFinal {
        home: i64,
        away: i64,
        /// 'home'/'away' do competidor marcado como `winner`, quando houver.
        winner_side: Option<String>,
        home_id: String,
        away_id: String,
        status_name: String,
        /// Empate no tempo normal/prorrogação decidido nos pênaltis.
        went_to_penalties: bool,
    },
    /// Não começou ou sem dados — ignorar.
    Skip,
}

/// Minuto-base extraído do relógio do provedor. Pega só os dígitos iniciais, para
/// que acréscimos como "45'+3'" virem 45 (e não 453).
pub(crate) fn live_elapsed(clock: &str) -> Option<i64> {
    let digits: String = clock.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<i64>().ok()
}

/// Rótulo amigável da fase do jogo ao vivo. Detecta intervalo, prorrogação e
/// pênaltis pelo status do provedor; caso contrário, mostra o minuto do relógio.
pub(crate) fn live_label(status_name: &str, clock: &str, short_detail: &str) -> String {
    match status_name {
        "STATUS_HALFTIME" => "Intervalo".to_string(),
        "STATUS_END_OF_REGULATION" => "Fim do 2º tempo".to_string(),
        "STATUS_EXTRA_TIME_HALFTIME" => "Intervalo da prorrogação".to_string(),
        "STATUS_PENALTIES" | "STATUS_SHOOTOUT" => "Pênaltis".to_string(),
        name if name.contains("EXTRA_TIME") => {
            if clock.is_empty() {
                "Prorrogação".to_string()
            } else {
                format!("Prorrogação · {clock}")
            }
        }
        _ if !clock.is_empty() => clock.to_string(),
        _ if !short_detail.is_empty() => short_detail.to_string(),
        _ => "Ao vivo".to_string(),
    }
}

/// Decide, de forma pura e testável, o que fazer com um evento externo.
pub(crate) fn classify_event(is_knockout: bool, event: &Event) -> GameApply {
    let Some(comp) = event.competition() else {
        return GameApply::Skip;
    };
    let (Some(home), Some(away)) = (comp.side("home"), comp.side("away")) else {
        return GameApply::Skip;
    };
    let home_score = parse_score(&home.score);
    let away_score = parse_score(&away.score);
    let state = comp.status.type_.state.as_str();
    let finished = state == "post" || comp.status.type_.completed;

    if state == "in" && !finished {
        let clock = comp.status.display_clock.trim();
        return GameApply::Live {
            home: home_score,
            away: away_score,
            elapsed: live_elapsed(clock),
            status: live_label(
                comp.status.type_.name.as_str(),
                clock,
                comp.status.type_.short_detail.trim(),
            ),
        };
    }

    if !finished {
        return GameApply::Skip;
    }

    if is_knockout {
        let winner_side = if home.winner {
            Some("home".to_string())
        } else if away.winner {
            Some("away".to_string())
        } else {
            None
        };
        return GameApply::KnockoutFinal {
            home: home_score,
            away: away_score,
            winner_side,
            home_id: home.team.id.clone(),
            away_id: away.team.id.clone(),
            status_name: comp.status.type_.name.clone(),
            went_to_penalties: comp.status.type_.name == "STATUS_FINAL_PEN",
        };
    }

    GameApply::FinalGroup {
        home: home_score,
        away: away_score,
        raw_status: comp.status.type_.name.clone(),
    }
}
