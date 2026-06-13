use dioxus::prelude::*;

use crate::auth::{self, AuthState};
use crate::AuthPendingCard;
use crate::Route;

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
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

        match auth::login(username(), password()).await {
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
            h1 { "Entrar" }
            p { class: "form-subtitle", "Acesse sua conta para ver seus bolões." }
            if !error_message().is_empty() {
                div { class: "error-banner", "{error_message()}" }
            }
            form {
                onsubmit: onsubmit,
                div {
                    class: "field",
                    input {
                        r#type: "text",
                        placeholder: "Usuário ou email",
                        value: username(),
                        oninput: move |e| username.set(e.value()),
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
                button {
                    class: "btn btn-primary",
                    r#type: "submit",
                    disabled: is_loading(),
                    if is_loading() { "Entrando..." } else { "Entrar" }
                }
            }
            p {
                class: "form-footer",
                "Não tem conta? "
                Link { to: Route::Register {}, "Registre-se aqui" }
            }
            p {
                class: "form-footer",
                Link { to: Route::ForgotPassword {}, "Esqueci minha senha" }
            }
        }
    }
}
