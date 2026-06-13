use dioxus::prelude::*;

use crate::auth::{self, AuthState};
use crate::AuthPendingCard;
use crate::Route;

#[component]
pub fn ForgotPasswordPage() -> Element {
    let mut email = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    // false = pedir o codigo; true = digitar codigo + nova senha.
    let mut awaiting_code = use_signal(|| false);
    let mut info_message = use_signal(String::new);
    let mut error_message = use_signal(String::new);
    let mut is_loading = use_signal(|| false);

    let auth_state = use_context::<Signal<AuthState>>();
    let navigator = use_navigator();

    let current_auth = auth_state();
    if current_auth.loading {
        return rsx! { AuthPendingCard { message: "Verificando sua sessão no Presumidos...".to_string() } };
    }

    let on_request = move |evt: FormEvent| {
        evt.prevent_default();

        async move {
            is_loading.set(true);
            error_message.set(String::new());

            match auth::request_password_reset(email()).await {
                Ok(()) => {
                    awaiting_code.set(true);
                    info_message.set(
                        "Se esse email estiver cadastrado, enviamos um código de 6 dígitos.".to_string(),
                    );
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

            match auth::confirm_password_reset(email(), code(), password()).await {
                Ok(()) => {
                    navigator.push(Route::Login {});
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
            h1 { "Recuperar senha" }
            if !error_message().is_empty() {
                div { class: "error-banner", "{error_message()}" }
            }
            if awaiting_code() {
                if !info_message().is_empty() {
                    p { class: "form-subtitle", "{info_message()}" }
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
                    div {
                        class: "field",
                        input {
                            r#type: "password",
                            placeholder: "Nova senha",
                            value: password(),
                            oninput: move |e| password.set(e.value()),
                            required: true,
                        }
                    }
                    div {
                        class: "field",
                        input {
                            r#type: "password",
                            placeholder: "Confirmar nova senha",
                            value: confirm_password(),
                            oninput: move |e| confirm_password.set(e.value()),
                            required: true,
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        disabled: is_loading(),
                        if is_loading() { "Redefinindo..." } else { "Redefinir senha" }
                    }
                }
            } else {
                p { class: "form-subtitle", "Informe seu email para receber um código de redefinição." }
                form {
                    onsubmit: on_request,
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
                    button {
                        class: "btn btn-primary",
                        r#type: "submit",
                        disabled: is_loading(),
                        if is_loading() { "Enviando código..." } else { "Enviar código" }
                    }
                }
            }
            p {
                class: "form-footer",
                "Lembrou a senha? "
                Link { to: Route::Login {}, "Voltar ao login" }
            }
        }
    }
}
