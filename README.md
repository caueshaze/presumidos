# Presumidos

Bolão da Copa do Mundo FIFA 2026: **backend Rust (Axum) servindo uma API
REST/JSON + SPA React (Vite + Tailwind)**, com SQLite e deploy em Docker atrás
do Caddy.

> Este projeto passou por uma migração de **Dioxus Fullstack** para a arquitetura
> atual: **API Rust desacoplada + frontend React**. A pasta `ferrugem-web/`
> mantém o nome histórico do crate do backend.

## Estado atual

O projeto já entrega:

- cadastro e login com **verificação por e-mail** (Resend) e reset de senha
- sessão no backend em cookie `HttpOnly` + token CSRF em toda mutação
- criação e entrada em bolões por código de convite
- palpites de partidas (fase de grupos e mata-mata, com classificado e pênaltis)
- lançamento de resultado oficial por administrador
- ajuste manual de pontos pelo organizador do bolão ou admin
- visualização dos palpites dos membros do bolão
- ranking por bolão com a regra de pontuação do projeto
- reautenticação de admin para ações sensíveis + trilha de auditoria
- liberação controlada do mata-mata (oculto até o admin liberar)
- **integração de resultados ao vivo** a partir de uma fonte pública de
  terceiros: preenche o resultado final dos jogos de grupo automaticamente
- migrations SQLite com seed das 104 partidas oficiais

## Arquitetura

```text
.
├── Cargo.toml                # workspace Rust
├── Dockerfile                # build multi-stage (frontend + backend)
├── docker-compose.yml        # ferrugem-web + redis + caddy
├── deploy/                   # Caddyfile, scripts de deploy/backup/restore
├── ferrugem-web/             # backend Rust (Axum + SQLite)
│   ├── migrations/           # schema SQLite + seed de partidas
│   └── src/
└── web/                      # frontend React (Vite + Tailwind)
    └── src/
```

- **Internet → Caddy (`:80/:443`) → `ferrugem-web:8080`.** O backend serve a API
  em `/api` e, em produção, também os arquivos estáticos da SPA (build do `web/`).
- Em **desenvolvimento**, o Vite (`:5173`) serve o frontend e faz proxy de `/api`
  para o backend em `:8080` (cookie de sessão same-origin, sem CORS).

### Backend (`ferrugem-web/src`)

- `main.rs`: bootstrap do servidor Axum, serve `/api` + SPA, comandos CLI
  (`bootstrap-admin`, `sync-fixtures`) e spawn do poller de resultados
- `api.rs`: rotas HTTP/JSON e handlers sob `/api`
- `auth.rs`: autenticação, sessões, hashing Argon2id, bootstrap de admin
- `pools.rs`: bolões, membros e ajustes de pontos
- `matches.rs`: partidas, palpites e resultado oficial
- `scoring.rs`: cálculo do ranking
- `football.rs`: integração de resultados ao vivo (poller + `sync-fixtures`)
- `email.rs`: envio de e-mails de verificação/recuperação via Resend
- `security.rs`: headers, CSRF, rate limit, resolução de IP/proxy e auditoria
- `config.rs`: carregamento e validação do `.env`
- `db.rs`: pool SQLite (WAL) e execução das migrations
- `models.rs`: tipos compartilhados (serde `camelCase` para o frontend)
- `context.rs` / `error.rs`: contexto por request e tipos de erro

### Frontend (`web/src`)

- **React 18 + TypeScript + Vite + Tailwind CSS**
- **TanStack Query** para data fetching/cache, **React Router** para navegação,
  **Framer Motion** para animação e **lucide-react** para ícones
- `pages/` (telas), `components/` (UI e `MatchCard`), `hooks/queries.ts`
  (chamadas à API), `types/` (espelham os models Rust) e `lib/` (utilitários)

## Como rodar (desenvolvimento)

Pré-requisitos: **Rust** e **Node.js 18+**.

```bash
cp .env.example .env
```

Em dev, o `.env` já vem com `APP_ENV=development` e `RATE_LIMIT_BACKEND=memory`
(dispensa Redis). O backend valida o `.env` no boot, cria o SQLite em
`DATABASE_PATH` se necessário e aplica as migrations automaticamente.
O suporte a web push ficou como feature opcional para não travar o `cargo run`
local no Windows com dependências TLS nativas; a imagem de produção continua
habilitando isso explicitamente no build.
Também existe `DEV_DISABLE_AUTH_EMAILS=true` para pular o envio de email de
cadastro/reset em desenvolvimento; os códigos aparecem no terminal.

**Terminal 1 — backend (API em `:8080`):**

```bash
cargo run -p ferrugem-web --features server
```

**Terminal 2 — frontend (Vite em `:5173`):**

```bash
cd web
npm install
npm run dev
```

Abra **http://localhost:5173**. O Vite faz proxy de `/api` para o backend.

## Validações

Backend:

```bash
cargo test --features server
cargo clippy --features server -- -D warnings
```

Frontend (a partir de `web/`):

```bash
npm run lint     # tsc --noEmit
npm run build    # tsc -b && vite build
```

## Banco de dados

SQLite em modo WAL (`PRAGMA journal_mode = WAL`, ver `ferrugem-web/src/db.rs`),
com as tabelas:

`users`, `sessions`, `pools`, `pool_members`, `matches`, `predictions`,
`app_settings`, `audit_logs`, `point_adjustments`, `pending_registrations`,
`password_reset_codes`.

Observações:

- o cadastro público nunca promove admin automaticamente
- o primeiro admin é criado por comando local de bootstrap com `ADMIN_BOOTSTRAP_SECRET`
- a sessão fica no backend e trafega em cookie `HttpOnly`; toda mutação usa token CSRF
- ações sensíveis de admin exigem confirmação recente de senha
- alterações administrativas críticas geram registro em `audit_logs`
- palpites são bloqueados após o kickoff da partida
- headers como `X-Forwarded-For`/`X-Real-IP`/`CF-Connecting-IP` só entram no rate
  limit e na auditoria quando o peer remoto pertence a `TRUSTED_PROXY_CIDRS`
- se `REQUIRE_TRUSTED_PROXY=true`, login/cadastro e endpoints autenticados recusam
  acesso direto fora do proxy confiável
- o rate limit usa `memory` em desenvolvimento e `redis` em produção; quando o
  Redis cai, `login`/`register`/confirmação de senha falham fechado, enquanto
  leituras degradam com log
- as 104 partidas oficiais são carregadas via migration, cada uma com `phase`
- o mata-mata fica oculto enquanto `app_settings.knockout_released = '0'`; o admin
  sempre vê tudo, monta os confrontos e libera de uma vez

## Resultados ao vivo

O backend pode preencher resultados automaticamente a partir de uma **fonte
pública de terceiros** (configurável via `FOOTBALL_API_BASE_URL`). Variáveis no
`.env`:

```bash
FOOTBALL_API_ENABLED=true        # liga a integração
FOOTBALL_POLLER_ENABLED=true     # sobe o poller (true em UMA instância só)
FOOTBALL_API_BASE_URL=<endpoint da fonte de placares>
FOOTBALL_POLL_INTERVAL_SECS=900  # 15 min
```

Como funciona:

- Um **poller em background** roda a cada `FOOTBALL_POLL_INTERVAL_SECS`. Para
  economizar requisições, ele só chama a API quando há jogo na janela (de −4h a
  +30min do kickoff).
- Quando um jogo de **fase de grupos** é marcado como encerrado, o poller grava
  o placar oficial (`result_source = 'api'`) e o ranking atualiza sozinho.
- O **mata-mata** é apenas exibido ao vivo (quando disponível): o resultado
  oficial (classificado/pênaltis) continua sendo lançado pelo admin.
- **Resultado manual é soberano:** o poller nunca sobrescreve um placar lançado
  pelo admin — em divergência, apenas registra `match_result_api_conflict` na
  auditoria.

Antes de usar, mapeie os jogos locais (`jogo-001..104`) aos ids da API **uma vez**
(grava `external_fixture_id`):

```bash
# pré-visualizar o casamento sem gravar
cargo run -p ferrugem-web --features server -- sync-fixtures --dry-run
# gravar
cargo run -p ferrugem-web --features server -- sync-fixtures --apply
# override manual de um mapeamento específico
cargo run -p ferrugem-web --features server -- sync-fixtures --fixture jogo-001=123
```

Em produção, rode o `sync-fixtures --apply` dentro do container (ver Deploy).

## Bootstrap do primeiro admin

O bootstrap inicial não fica exposto por rota HTTP nem por UI pública.

Fluxo: subir app/banco com `.env` → rodar o bootstrap uma vez → confirmar criação
→ rotacionar/remover `ADMIN_BOOTSTRAP_SECRET` → operar normalmente.

Em desenvolvimento, use o script (roda da raiz, carrega `.env` e `bolao.db`):

```bash
scripts/dev-admin.sh admin admin@local.dev
# ou, sem prompt de senha:
BOOTSTRAP_ADMIN_PASSWORD='senha-de-dev' scripts/dev-admin.sh admin admin@local.dev
```

> O bootstrap cria o *primeiro* admin e só funciona enquanto não houver nenhum.
> Para recomeçar o dev do zero: `rm -f bolao.db bolao.db-shm bolao.db-wal` e rode de novo.

Equivalente manual (senha interativa, ou via `BOOTSTRAP_ADMIN_PASSWORD`):

```bash
cargo run -p ferrugem-web --features server -- \
  bootstrap-admin --username admin --email admin@seudominio.com
```

`ADMIN_BOOTSTRAP_SECRET` autoriza o bootstrap; `BOOTSTRAP_ADMIN_PASSWORD` define a
senha apenas para esse comando (se ausente, é pedida interativamente).

## Deploy com Caddy

O repositório inclui:

- [Dockerfile](Dockerfile): build multi-stage — estágio Node compila o frontend
  (`web/` → `dist`), estágio Rust compila o backend com cache via `cargo-chef`, e
  a imagem final junta o binário com a SPA em `/app/public`
- [docker-compose.yml](docker-compose.yml): origin sem porta pública + Caddy como
  entrada + Redis interno para rate limit
- [deploy/Caddyfile](deploy/Caddyfile): proxy reverso com HTTPS automático
- [deploy/deploy.sh](deploy/deploy.sh): backup pré-deploy + build + restart + healthcheck

Desenho da rede:

- Internet → Caddy `:80/:443` → `ferrugem-web:8080`
- `ferrugem-web` → Redis interno para rate limit persistente
- o origin usa apenas `expose` (sem `ports`); a rede `origin` é interna e dedicada
- o Caddy também entra na rede `public` para emitir/renovar certificados
  (Let's Encrypt) e é o único publicado em `80/443` (e `443/udp` para HTTP/3)
- o Caddy tem IP fixo `172.31.0.10`; o app confia apenas em `TRUSTED_PROXY_CIDRS=172.31.0.10/32`

Ajustes no `.env` para produção:

- `APP_ENV=production`, `APP_DOMAIN=seu-dominio.com`, `COOKIE_SECURE=true`
- `CONTACT_EMAIL=contato@seu-dominio.com` para a página pública de contato
- `SESSION_SECRET` e `ADMIN_BOOTSTRAP_SECRET` fortes (32+ caracteres)
- `REQUIRE_TRUSTED_PROXY=true`
- `RATE_LIMIT_BACKEND=redis`, `REDIS_URL=redis://redis:6379`
- `RATE_LIMIT_IDENTITY_SECRET` próprio (32+ caracteres, separado do `SESSION_SECRET`)
- `RESEND_API_KEY` e `RESEND_FROM_EMAIL` para os e-mails transacionais

Subida e deploy:

```bash
docker compose build ferrugem-web
docker compose up -d
# fluxo recomendado de atualização na VPS:
./deploy/deploy.sh
```

O `deploy.sh` faz: backup pré-deploy do SQLite, `docker compose build` com
`DOCKER_BUILDKIT=1`, `up -d`, valida `GET /api/health` via Caddy e, em falha de
healthcheck, reaplica a imagem anterior automaticamente.

Depois do deploy, rode (uma vez) o bootstrap do admin e o mapeamento de jogos
dentro do container:

```bash
docker compose exec ferrugem-web \
  /app/ferrugem-web bootstrap-admin --username admin --email admin@seudominio.com

docker compose exec ferrugem-web \
  /app/ferrugem-web sync-fixtures --apply
```

O servidor também executa uma limpeza conservadora ao iniciar, removendo apenas
sessões expiradas, cadastros pendentes vencidos, códigos de reset vencidos e
dados antigos/inativos de web push. Para rodar isso manualmente:

```bash
docker compose exec ferrugem-web /app/ferrugem-web cleanup-expired
```

O proxy reconstrói `X-Forwarded-For`, `X-Real-IP` e `Forwarded` em vez de
encaminhar o que veio do cliente, e remove `CF-Connecting-IP`.

Observações de cache de build: alterações em `Cargo.toml`/`Cargo.lock` invalidam a
camada de dependências Rust (esperado); mudanças só em `ferrugem-web/src` ou em
`web/src` reaproveitam o cache. Evite `docker system prune -a`, que destrói o
benefício do cache.

## Backup e Restore

O banco SQLite (WAL) vive no volume Docker `app_data` (`/data/bolao.db`). Os
scripts usam uma imagem auxiliar (`deploy/backup/`, alpine + `sqlite3`) para
gerar backups consistentes com a aplicação rodando.

### Backup manual

```bash
./deploy/backup.sh
```

- roda `sqlite3 /data/bolao.db ".backup '...'"` (seguro com WAL, app ligada)
- salva em `./backups/ferrugem-YYYYMMDD-HHMMSS.db`, **fora** do volume `app_data`
- valida `PRAGMA integrity_check;` (se falhar, apaga o arquivo e retorna erro)
- aplica `chmod 600` e remove backups com mais de 14 dias

`./backups/` é criado com `chmod 700` e está no `.gitignore`.

### Backup automático (cron)

```cron
0 3 * * * cd /caminho/do/repo && ./deploy/backup.sh >> /var/log/ferrugem-backup.log 2>&1
```

### Cópia externa

```bash
rsync -av ./backups/ usuario@outro-host:/caminho/de/backups/ferrugem/
```

### Restore em produção

```bash
./deploy/restore.sh backups/ferrugem-20260612-030000.db
```

O script valida `integrity_check` **antes** de tocar em produção, pede confirmação
interativa, cria um backup pré-restore do estado atual e então faz
`docker compose down`, substitui os arquivos `bolao.db*` e sobe de novo.

### Restore testado (ambiente isolado)

```bash
./deploy/restore-test.sh backups/ferrugem-20260612-030000.db
```

Cria volume e container temporários (sem tocar em `app_data`/`origin`), sobe em
`http://localhost:18080` com `APP_ENV=development` e `RATE_LIMIT_BACKEND=memory`.
Ao encerrar com `Ctrl+C`, tudo é removido automaticamente.

## Checklist de fechamento do origin

1. `curl http://IP_DA_VPS:8080` falha ou não conecta.
2. `curl -I http://seu-dominio.com` redireciona para HTTPS.
3. `curl -I https://seu-dominio.com` responde pelo Caddy.
4. `docker compose ps` mostra apenas Caddy com portas publicadas.
5. `nmap` externo da VPS deve mostrar somente `22`, `80` e `443`.

O firewall da VPS ainda precisa ser fechado fora do repositório. Exemplo com `ufw`:

```bash
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw deny 8080/tcp
ufw enable
```

## Licença

Distribuído sob a licença **MIT**. Veja o arquivo [LICENSE](LICENSE) para os
termos completos.
