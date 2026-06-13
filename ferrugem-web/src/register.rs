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
    let mut code = use_signal(String::new);
    // false = formulario de dados; true = digitar codigo enviado por email.
    let mut awaiting_code = use_signal(|| false);
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

    let on_request = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
            is_loading.set(true);
            error_message.set(String::new());

            if password() != confirm_password() {
                error_message.set("As senhas não coincidem.".to_string());
                is_loading.set(false);
                return;
            }

            if password().len() < 8 {
                error_message.set("A senha deve ter pelo menos 8 caracteres.".to_string());
                is_loading.set(false);
                return;
            }

            match auth::request_registration(username(), email(), password()).await {
                Ok(()) => {
                    awaiting_code.set(true);
                }
                Err(ServerFnError::ServerError { message, .. }) => error_message.set(message),
                Err(e) => error_message.set(e.to_string()),
            }

            is_loading.set(false);
        }
    };

    let on_confirm = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
            is_loading.set(true);
            error_message.set(String::new());

            match auth::confirm_registration(email(), code()).await {
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
            if awaiting_code() {
                p { class: "form-subtitle", "Enviamos um código de 6 dígitos para {email()}. Digite-o abaixo para confirmar." }
                if !error_message().is_empty() {
                    div { class: "error-banner", "{error_message()}" }
                }
                form {
                    onsubmit: on_confirm,
                    div {
                        class: "field",
                        input {
                            r#type: "text",
                            inputmode: "numeric",
                            maxlength: "6",
                            placeholder: "Código de 6 dígitos",
                            value: code(),
                            oninput: move |e| code.set(e.value()),
                            required: true,
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        disabled: is_loading(),
                        if is_loading() { "Confirmando..." } else { "Confirmar conta" }
                    }
                }
                p {
                    class: "form-footer",
                    button {
                        class: "btn btn-link",
                        r#type: "button",
                        onclick: move |_| {
                            awaiting_code.set(false);
                            code.set(String::new());
                            error_message.set(String::new());
                        },
                        "Voltar e corrigir os dados"
                    }
                }
            } else {
                p { class: "form-subtitle", "Cadastre-se para criar ou entrar em bolões." }
                if !error_message().is_empty() {
                    div { class: "error-banner", "{error_message()}" }
                }
                form {
                    onsubmit: on_request,
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
                        if is_loading() { "Enviando código..." } else { "Criar conta" }
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
}
