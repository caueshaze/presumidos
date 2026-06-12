use dioxus::prelude::*;

use crate::auth::AuthState;
use crate::pools::list_my_pools;
use crate::scoring::get_leaderboard;
use crate::AuthPendingCard;
use crate::Route;

fn medal(position: usize) -> &'static str {
    match position {
        0 => "🥇",
        1 => "🥈",
        2 => "🥉",
        _ => "",
    }
}

#[component]
pub fn Leaderboard() -> Element {
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
        return rsx! { AuthPendingCard { message: "Montando o ranking do Presumidos...".to_string() } };
    }

    if auth_state.user.is_none() {
        return rsx! { AuthPendingCard { message: "Redirecionando para o login...".to_string() } };
    }

    let pools = use_resource(move || {
        let token = auth().token.clone();
        async move { list_my_pools(token).await }
    });

    let mut selected_pool = use_signal(String::new);

    use_effect(move || {
        if selected_pool().is_empty() {
            if let Some(Ok(list)) = pools.value()() {
                if let Some(first) = list.first() {
                    selected_pool.set(first.id.clone());
                }
            }
        }
    });

    let entries = use_resource(move || {
        let token = auth().token.clone();
        let pool_id = selected_pool();
        async move {
            if pool_id.is_empty() {
                Ok(Vec::new())
            } else {
                get_leaderboard(token, pool_id).await
            }
        }
    });

    rsx! {
        div {
            class: "page",
            h1 { "Ranking" }
            p { class: "ranking-intro",
                "A pontuação considera o placar do tempo normal. Placar exato vale 7 pontos; "
                "resultado correto vale 3; acertar os gols de um time que marcou pelo menos 1 gol dá +1. "
                "No mata-mata, acertar o classificado dá +2, e palpites corretos sobre pênaltis podem render bônus extras."
            }

            {
                match pools.value()() {
                    Some(Ok(list)) if !list.is_empty() => rsx! {
                        div {
                            class: "card pool-selector",
                            div {
                                class: "field",
                                label { r#for: "pool-select", "Bolão" }
                                select {
                                    id: "pool-select",
                                    value: "{selected_pool()}",
                                    onchange: move |e| selected_pool.set(e.value()),
                                    for p in list {
                                        option { value: "{p.id}", "{p.name}" }
                                    }
                                }
                            }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div {
                            class: "card ranking-empty-state",
                            h3 { "Seu pódio ainda está no aquecimento" }
                            p { "Crie um bolão ou entre com um código e deixe a disputa começar." }
                        }
                    },
                    Some(Err(e)) => rsx! {
                        div { class: "error-banner", "Erro ao carregar bolões: {e}" }
                    },
                    None => rsx! {
                        div { class: "card", p { "Carregando..." } }
                    },
                }
            }

            {
                match pools.value()() {
                    Some(Ok(list)) if list.is_empty() => rsx! {},
                    _ => rsx! {
                        match entries.value()() {
                            Some(Ok(list)) if !list.is_empty() => {
                                let podium: Vec<_> = list.iter().take(3).cloned().collect();
                                let rest: Vec<_> = list.iter().skip(3).cloned().collect();

                                rsx! {
                                    div {
                                        class: "podium",
                                        for (i, entry) in podium.into_iter().enumerate() {
                                            div {
                                                class: "podium-place podium-{i + 1}",
                                                span { class: "medal", "{medal(i)}" }
                                                div { "{entry.username}" }
                                                div { class: "points", "{entry.points} pts" }
                                            }
                                        }
                                    }
                                    if !rest.is_empty() {
                                        table {
                                            class: "leaderboard-table",
                                            thead {
                                                tr {
                                                    th { "Posição" }
                                                    th { "Usuário" }
                                                    th { "Pontos" }
                                                }
                                            }
                                            tbody {
                                                for (i, entry) in rest.into_iter().enumerate() {
                                                    tr {
                                                        td { "{i + 4}" }
                                                        td { "{entry.username}" }
                                                        td { "{entry.points}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some(Ok(_)) => rsx! {
                                div {
                                    class: "card ranking-empty-state",
                                    h3 { "Ainda ninguém balançou esse ranking" }
                                    p { "Quando os resultados oficiais entrarem, a tabela ganha vida por aqui." }
                                }
                            },
                            Some(Err(e)) => rsx! {
                                div { class: "error-banner", "Erro ao carregar ranking: {e}" }
                            },
                            None => rsx! {
                                div { class: "card", p { "Carregando..." } }
                            },
                        }
                    }
                }
            }
        }
    }
}
