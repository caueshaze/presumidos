use dioxus::prelude::*;

mod auth;
mod models;
mod login;
mod register;
mod dashboard;
mod pools;
mod matches;
mod predictions;
mod scoring;
mod leaderboard;

#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod security;

use crate::auth::AuthState;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(Navbar)]
    #[route("/")]
    Home {},
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
    #[route("/dashboard")]
    Dashboard {},
    #[route("/predictions")]
    Predictions {},
    #[route("/leaderboard")]
    Leaderboard {},
}

// Importar os novos componentes
use crate::login::LoginPage;
use crate::register::RegisterPage;
use crate::dashboard::Dashboard;
use crate::predictions::Predictions;
use crate::leaderboard::Leaderboard;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(feature = "server")]
    {
        let rt = tokio::runtime::Runtime::new().expect("falha ao criar runtime tokio");
        rt.block_on(db::init());
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    #[cfg(feature = "server")]
    crate::security::apply_security_headers();

    let mut auth = use_context_provider(|| Signal::new(AuthState::default()));
    let mut auth_bootstrapped = use_signal(|| false);

    use_effect(move || {
        if auth_bootstrapped() {
            return;
        }

        auth_bootstrapped.set(true);

        spawn(async move {
            let legacy_token = auth::load_token().await;

            match auth::current_user(legacy_token.clone()).await {
                Ok(session) => {
                    if session.user.is_some() {
                        auth::clear_token().await;
                    }
                    auth.set(AuthState {
                        user: session.user,
                        token: String::new(),
                        csrf_token: session.csrf_token,
                        loading: false,
                    });
                }
                Err(_) => {
                    auth::clear_token().await;
                    auth.set(AuthState {
                        user: None,
                        token: String::new(),
                        csrf_token: String::new(),
                        loading: false,
                    });
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Fredoka:wght@500;600;700&family=Nunito+Sans:wght@400;600;700&display=swap"
        }
        Router::<Route> {}
    }
}

#[component]
pub fn AuthPendingCard(message: String) -> Element {
    rsx! {
        div {
            class: "page",
            div {
                class: "card auth-pending",
                div { class: "auth-pending-spinner", aria_hidden: "true" }
                p { "{message}" }
            }
        }
    }
}

#[component]
pub fn Hero() -> Element {
    let auth = use_context::<Signal<AuthState>>();

    rsx! {
        div {
            id: "hero",
            span { class: "hero-kicker", "Presumidos" }
            h1 { "⚽ Presumidos da Copa 2026" }
            p { "O bolão Presumidos transforma cada palpite em disputa, resenha e ranking entre amigos." }
            div {
                class: "hero-actions",
                if auth().user.is_some() {
                    Link { class: "btn btn-primary", to: Route::Dashboard {}, "Entrar no Presumidos" }
                    Link { class: "btn btn-secondary", to: Route::Predictions {}, "Meus palpites" }
                } else {
                    Link { class: "btn btn-primary", to: Route::Register {}, "Criar conta no Presumidos" }
                    Link { class: "btn btn-secondary", to: Route::Login {}, "Entrar para acompanhar" }
                }
            }
        }
    }
}

#[component]
fn PublicBenefits() -> Element {
    rsx! {
        div {
            class: "feature-grid",
            div {
                class: "card feature-card",
                span { class: "feature-icon", "⚔️" }
                h3 { "Competição boa de verdade" }
                p { "Cada rodada vira uma disputa divertida entre amigos, sem complicação para entrar e começar." }
            }
            div {
                class: "card feature-card",
                span { class: "feature-icon", "📈" }
                h3 { "Ranking sempre à vista" }
                p { "Acompanhe quem está dominando o bolão com uma leitura rápida do pódio e da tabela." }
            }
            div {
                class: "card feature-card",
                span { class: "feature-icon", "🎯" }
                h3 { "Palpite rápido e direto" }
                p { "Salve seus placares com poucos cliques e foque no jogo, não no formulário." }
            }
        }
    }
}

#[component]
fn PublicTeaser() -> Element {
    rsx! {
        div {
            class: "teaser-showcase",
            div {
                class: "card teaser-card teaser-card-prediction",
                span { class: "teaser-label", "Preview de palpite" }
                h3 { "Brasil vs Argentina" }
                p { class: "teaser-meta", "Domingo, 18:00" }
                div {
                    class: "teaser-scoreline",
                    div { class: "teaser-scorebox", "2" }
                    span { class: "teaser-score-separator", "x" }
                    div { class: "teaser-scorebox", "1" }
                }
                p { class: "teaser-caption", "No Presumidos, o palpite fica claro, leve e pronto para a disputa." }
            }
            div {
                class: "card teaser-card teaser-card-ranking",
                span { class: "teaser-label", "Preview de ranking" }
                h3 { "Pódio da rodada" }
                div {
                    class: "teaser-podium",
                    div { class: "teaser-podium-item second", span { "🥈" } strong { "Bia" } small { "13 pts" } }
                    div { class: "teaser-podium-item first", span { "🥇" } strong { "Caue" } small { "17 pts" } }
                    div { class: "teaser-podium-item third", span { "🥉" } strong { "Luca" } small { "11 pts" } }
                }
                p { class: "teaser-caption", "Visual de produto, clima de resenha e leitura rápida de quem está na frente." }
            }
        }
    }
}

/// Home page
#[component]
fn Home() -> Element {
    let auth = use_context::<Signal<AuthState>>();

    rsx! {
        div {
            class: "page",
            Hero {}
            if auth().user.is_none() {
                div {
                    class: "landing-shell",
                    div {
                        id: "home-content",
                        class: "card landing-copy",
                        h2 { "Entre, chame a galera e deixe o ranking falar" }
                        p { "📝 Cadastre-se, crie seu bolão no Presumidos ou entre em um convite já criado." }
                        p { "🔮 Salve seus palpites antes do apito inicial e acompanhe tudo sem fricção." }
                        p { "🏆 Quando os resultados oficiais entram, o ranking se atualiza e a resenha começa." }
                    }
                    PublicBenefits {}
                    PublicTeaser {}
                }
            } else {
                div {
                    id: "home-content",
                    class: "card landing-copy",
                    h2 { "Você já está dentro do Presumidos" }
                    p { "Acesse seus bolões, salve seus palpites e acompanhe o ranking de cada disputa." }
                    div {
                        class: "hero-actions hero-actions-inline",
                        Link { class: "btn btn-primary", to: Route::Dashboard {}, "Ir para meus bolões" }
                        Link { class: "btn btn-secondary", to: Route::Leaderboard {}, "Ver ranking" }
                    }
                }
            }
        }
    }
}

/// Login page
#[component]
fn Login() -> Element {
    rsx! {
        LoginPage {}
    }
}

/// Register page
#[component]
fn Register() -> Element {
    rsx! {
        RegisterPage {}
    }
}

/// Shared navbar component.
#[component]
fn Navbar() -> Element {
    let mut auth = use_context::<Signal<AuthState>>();
    let navigator = use_navigator();
    let current_route: Route = use_route();

    let logout = move |_| {
        spawn(async move {
            let token = auth().token.clone();
            let csrf_token = auth().csrf_token.clone();
            let _ = auth::logout(token, csrf_token).await;
            auth::clear_token().await;
            auth.set(AuthState {
                user: None,
                token: String::new(),
                csrf_token: String::new(),
                loading: false,
            });
            navigator.push(Route::Home {});
        });
    };

    let auth_state = auth();
    let is_home = matches!(current_route, Route::Home {});
    let is_dashboard = matches!(current_route, Route::Dashboard {});
    let is_predictions = matches!(current_route, Route::Predictions {});
    let is_leaderboard = matches!(current_route, Route::Leaderboard {});
    let is_login = matches!(current_route, Route::Login {});
    let is_register = matches!(current_route, Route::Register {});

    rsx! {
        div {
            id: "navbar",
            Link {
                class: if is_home { "navbar-brand active-nav" } else { "navbar-brand" },
                to: Route::Home {},
                "Presumidos"
            }
            if auth_state.user.is_some() {
                Link {
                    class: if is_dashboard { "active-nav" } else { "" },
                    to: Route::Dashboard {},
                    "Dashboard"
                }
                Link {
                    class: if is_predictions { "active-nav" } else { "" },
                    to: Route::Predictions {},
                    "Previsões"
                }
                Link {
                    class: if is_leaderboard { "active-nav" } else { "" },
                    to: Route::Leaderboard {},
                    "Ranking"
                }
                div { class: "navbar-spacer" }
                if let Some(user) = auth_state.user {
                    span { class: "navbar-user", "Olá, {user.username}" }
                }
                a { class: "btn btn-outline", onclick: logout, "Sair" }
            } else {
                div { class: "navbar-spacer" }
                if !auth_state.loading {
                    Link {
                        class: if is_login { "btn btn-secondary navbar-cta active-cta" } else { "btn btn-secondary navbar-cta" },
                        to: Route::Login {},
                        "Login"
                    }
                    Link {
                        class: if is_register { "btn btn-primary navbar-cta active-cta" } else { "btn btn-primary navbar-cta" },
                        to: Route::Register {},
                        "Criar conta"
                    }
                }
            }
        }

        Outlet::<Route> {}
    }
}
