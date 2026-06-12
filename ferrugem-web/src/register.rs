use dioxus::prelude::*;

use crate::auth::{self, AuthState};
use crate::AuthPendingCard;
use crate::Route;

#[component]
pub fn RegisterPage() -> Element {
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut error_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    let mut auth_state = use_context::<Signal<AuthState>>();
    let navigator = use_navigator();

    use_effect(move || {
        let state = auth_state();
        if !state.loading && state.user.is_some() {
            navigator.push(Route::Dashboard {});
        }
    });

    let current_auth = auth_state();
    if current_auth.loading {
        return rsx! { AuthPendingCard { message: "Verificando sua sessão no Presumidos...".to_string() } };
    }

    if current_auth.user.is_some() {
        return rsx! { AuthPendingCard { message: "Redirecionando para o seu painel...".to_string() } };
    }

    let onsubmit = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
        is_loading.set(true);
        error_message.set(String::new());

        if password() != confirm_password() {
            error_message.set("As senhas não coincidem.".to_string());
            is_loading.set(false);
            return;
        }

        if password().len() < 6 {
            error_message.set("A senha deve ter pelo menos 6 caracteres.".to_string());
            is_loading.set(false);
            return;
        }

        match auth::register(username(), email(), password()).await {
            Ok(result) => {
                auth_state.set(AuthState {
                    user: Some(result.user),
                    token: String::new(),
                    csrf_token: result.csrf_token,
                    loading: false,
                });
                navigator.push(Route::Dashboard {});
            }
            Err(ServerFnError::ServerError { message, .. }) => error_message.set(message),
            Err(e) => error_message.set(e.to_string()),
        }

        is_loading.set(false);
        }
    };

    rsx! {
        div {
            class: "page form-card card",
            h1 { "Criar conta" }
            p { class: "form-subtitle", "Cadastre-se para criar ou entrar em bolões." }
            if !error_message().is_empty() {
                div { class: "error-banner", "{error_message()}" }
            }
            form {
                onsubmit: onsubmit,
                div {
                    class: "field",
                    input {
                        r#type: "text",
                        placeholder: "Nome de usuário",
                        value: username(),
                        oninput: move |e| username.set(e.value()),
                        required: true,
                    }
                }
                div {
                    class: "field",
                    input {
                        r#type: "email",
                        placeholder: "Email",
                        value: email(),
                        oninput: move |e| email.set(e.value()),
                        required: true,
                    }
                }
                div {
                    class: "field",
                    input {
                        r#type: "password",
                        placeholder: "Senha",
                        value: password(),
                        oninput: move |e| password.set(e.value()),
                        required: true,
                    }
                }
                div {
                    class: "field",
                    input {
                        r#type: "password",
                        placeholder: "Confirmar senha",
                        value: confirm_password(),
                        oninput: move |e| confirm_password.set(e.value()),
                        required: true,
                    }
                }
                button {
                    class: "btn btn-primary",
                    r#type: "submit",
                    disabled: is_loading(),
                    if is_loading() { "Registrando..." } else { "Criar conta" }
                }
            }
            p {
                class: "form-footer",
                "Já tem conta? "
                Link { to: Route::Login {}, "Faça login aqui" }
            }
        }
    }
}
