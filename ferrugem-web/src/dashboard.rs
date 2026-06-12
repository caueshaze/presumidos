use dioxus::prelude::*;

use crate::auth::AuthState;
use crate::pools::{create_pool, join_pool, list_my_pools};
use crate::AuthPendingCard;
use crate::Route;

#[component]
pub fn Dashboard() -> Element {
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
        return rsx! { AuthPendingCard { message: "Verificando sua sessão no Presumidos...".to_string() } };
    }

    if auth_state.user.is_none() {
        return rsx! { AuthPendingCard { message: "Redirecionando para o login...".to_string() } };
    }

    let mut pools = use_resource(move || {
        let token = auth().token.clone();
        async move { list_my_pools(token).await }
    });

    let mut new_pool_name = use_signal(String::new);
    let mut join_code = use_signal(String::new);
    let mut form_error = use_signal(String::new);
    let mut form_loading = use_signal(|| false);

    let create = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
        form_loading.set(true);
        form_error.set(String::new());

        match create_pool(auth().token.clone(), new_pool_name(), auth().csrf_token.clone()).await {
            Ok(_) => {
                new_pool_name.set(String::new());
                pools.restart();
            }
            Err(ServerFnError::ServerError { message, .. }) => form_error.set(message),
            Err(e) => form_error.set(e.to_string()),
        }

        form_loading.set(false);
        }
    };

    let join = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
        form_loading.set(true);
        form_error.set(String::new());

        match join_pool(auth().token.clone(), join_code(), auth().csrf_token.clone()).await {
            Ok(_) => {
                join_code.set(String::new());
                pools.restart();
            }
            Err(ServerFnError::ServerError { message, .. }) => form_error.set(message),
            Err(e) => form_error.set(e.to_string()),
        }

        form_loading.set(false);
        }
    };

    rsx! {
        div {
            class: "page",
            h1 { "Seus bolões" }
            if let Some(user) = auth().user {
                p { "Bem-vindo, {user.username}! Crie um bolão ou entre com um código de convite para começar." }
            }

            if !form_error().is_empty() {
                div { class: "error-banner", "{form_error()}" }
            }

            if let Some(result) = pools.value()() {
                if let Ok(list) = result {
                    if list.is_empty() {
                        div {
                            class: "card dashboard-empty-state",
                            h2 { "Seu painel começa aqui" }
                            p { "Você ainda não participa de nenhum bolão no Presumidos." }
                            p { "Crie um bolão para reunir a galera ou entre com um código para começar a palpitar." }
                        }
                    } else {
                        div {
                            class: "card-grid",
                            for p in list {
                                div {
                                    class: "card pool-card",
                                    h3 { "{p.name}" }
                                    span {
                                        class: "invite-code",
                                        "Código: {p.invite_code}"
                                    }
                                    p { "{p.member_count} membro(s)" }
                                    div {
                                        class: "pool-actions",
                                        Link { class: "btn btn-primary", to: Route::Predictions {}, "Palpites" }
                                        Link { class: "btn btn-secondary", to: Route::Leaderboard {}, "Ranking" }
                                    }
                                }
                            }
                        }
                    }
                } else if let Err(e) = result {
                    div { class: "error-banner", "Erro ao carregar bolões: {e}" }
                }
            } else {
                div { class: "card", p { "Carregando..." } }
            }

            div {
                class: "dashboard-forms",
                div {
                    class: "card",
                    h2 { "Criar bolão" }
                    form {
                        onsubmit: create,
                        div {
                            class: "field",
                            input {
                                r#type: "text",
                                placeholder: "Nome do bolão",
                                value: new_pool_name(),
                                oninput: move |e| new_pool_name.set(e.value()),
                                required: true,
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            r#type: "submit",
                            disabled: form_loading(),
                            "Criar"
                        }
                    }
                }
                div {
                    class: "card",
                    h2 { "Entrar com código" }
                    form {
                        onsubmit: join,
                        div {
                            class: "field",
                            input {
                                r#type: "text",
                                placeholder: "Código de convite",
                                value: join_code(),
                                oninput: move |e| join_code.set(e.value()),
                                required: true,
                            }
                        }
                        button {
                            class: "btn btn-secondary",
                            r#type: "submit",
                            disabled: form_loading(),
                            "Entrar"
                        }
                    }
                }
            }
        }
    }
}
