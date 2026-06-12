# Ferrugem Web

Aplicação de bolão da Copa 2026 feita em Rust com Dioxus Fullstack e SQLite.

## Estado atual

O projeto já entrega:

- cadastro e login com sessão em cookie `HttpOnly`
- criação e entrada em bolões por código de convite
- listagem de partidas e envio de palpites
- lançamento de resultado oficial por administrador
- reautenticação de admin para ações sensíveis e trilha de auditoria
- ranking por bolão com a regra atual do projeto
- migrations SQLite com seed inicial de partidas

## Estrutura

Este repositório é um workspace Rust com um app principal:

```text
.
├── Cargo.toml
├── Cargo.lock
└── ferrugem-web/
    ├── Cargo.toml
    ├── Dioxus.toml
    ├── assets/
    ├── migrations/
    └── src/
```

Arquivos importantes:

- `ferrugem-web/src/main.rs`: rotas, layout e bootstrap do app
- `ferrugem-web/src/auth.rs`: autenticação, sessão e helpers de browser
- `ferrugem-web/src/pools.rs`: criação/entrada em bolões
- `ferrugem-web/src/matches.rs`: partidas, palpites e resultado oficial
- `ferrugem-web/src/scoring.rs`: cálculo do ranking
- `ferrugem-web/migrations/`: schema SQLite e seed inicial

## Como rodar

Pré-requisitos:

- Rust instalado
- Dioxus CLI: `cargo install dioxus-cli`

Rodar em desenvolvimento:

```bash
cp .env.example .env
cd ferrugem-web
dx serve
```

O arquivo `.env` e obrigatorio para subir o servidor. O app valida `APP_ENV`, `DATABASE_PATH`, `SESSION_SECRET`, `SESSION_TTL_HOURS`, `COOKIE_SECURE` e `ADMIN_REAUTH_TTL_MINUTES` logo no boot, cria o SQLite nesse caminho se necessario e aplica as migrations automaticamente no modo server.

## Features e targets

O app usa estas features:

- `web`: frontend Dioxus para navegador
- `server`: server functions + SQLite + autenticação
- `desktop`: definido no manifesto, mas não é o foco atual do projeto

Validações úteis:

```bash
cargo check
cargo test --features server
cargo test --no-default-features --features web
cargo clippy --features server -- -D warnings
cargo clippy --no-default-features --features web -- -D warnings
```

## Banco de dados

O banco atual é SQLite e usa as tabelas:

- `users`
- `sessions`
- `pools`
- `pool_members`
- `matches`
- `predictions`
- `app_settings`

Observações:

- o primeiro usuário registrado vira administrador
- a sessão fica no backend e trafega em cookie `HttpOnly`
- toda mutação autenticada usa token CSRF de sessão
- ações sensíveis de admin exigem confirmação recente de senha
- alterações administrativas críticas geram registro em `audit_logs`
- palpites são bloqueados após o kickoff da partida
- `DATABASE_PATH` precisa existir no `.env`
- as 104 partidas oficiais da Copa 2026 são carregadas via migration; cada uma tem `phase` (fase de grupos, 16 avos, oitavas, etc.)
- o mata-mata fica oculto para os participantes enquanto `app_settings.knockout_released = '0'`; o admin sempre vê todos os jogos, monta os confrontos e libera tudo de uma vez pelo botão "Liberar mata-mata" na página de palpites
