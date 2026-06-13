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

O arquivo `.env` e obrigatorio para subir o servidor. O app valida `APP_ENV`, `DATABASE_PATH`, `SESSION_SECRET`, `ADMIN_BOOTSTRAP_SECRET`, `SESSION_TTL_HOURS`, `COOKIE_SECURE`, `ADMIN_REAUTH_TTL_MINUTES`, `TRUSTED_PROXY_CIDRS`, `REQUIRE_TRUSTED_PROXY`, `RATE_LIMIT_BACKEND` e `REDIS_URL` logo no boot, cria o SQLite nesse caminho se necessario e aplica as migrations automaticamente no modo server.

`APP_DOMAIN` e usado pelo proxy reverso do deploy Docker. O app Rust ignora essa variavel.

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

- o cadastro público nunca promove admin automaticamente
- o primeiro admin precisa ser criado por comando local de bootstrap com `ADMIN_BOOTSTRAP_SECRET`
- a sessão fica no backend e trafega em cookie `HttpOnly`
- toda mutação autenticada usa token CSRF de sessão
- ações sensíveis de admin exigem confirmação recente de senha
- alterações administrativas críticas geram registro em `audit_logs`
- palpites são bloqueados após o kickoff da partida
- `DATABASE_PATH` precisa existir no `.env`
- headers como `X-Forwarded-For`, `X-Real-IP` e `CF-Connecting-IP` so entram no rate limit e na auditoria quando o peer remoto real pertence a `TRUSTED_PROXY_CIDRS`
- se `REQUIRE_TRUSTED_PROXY=true`, as server functions autenticadas e de login/cadastro recusam acesso direto fora do proxy confiável
- o rate limit usa `memory` no desenvolvimento e `redis` em producao
- quando o backend Redis de rate limit cair, `login`, `register` e `confirm_admin_password` falham fechado; `current_user` e `join_pool` degradam com log e seguem
- as 104 partidas oficiais da Copa 2026 são carregadas via migration; cada uma tem `phase` (fase de grupos, 16 avos, oitavas, etc.)
- o mata-mata fica oculto para os participantes enquanto `app_settings.knockout_released = '0'`; o admin sempre vê todos os jogos, monta os confrontos e libera tudo de uma vez pelo botão "Liberar mata-mata" na página de palpites

## Bootstrap do primeiro admin

O bootstrap inicial de admin nao fica exposto por rota HTTP nem por UI publica.

Fluxo operacional:

1. subir app e banco com `.env` configurado
2. rodar o comando local de bootstrap uma unica vez
3. confirmar que o admin foi criado
4. rotacionar ou remover `ADMIN_BOOTSTRAP_SECRET`
5. seguir a operacao normal

Exemplo recomendado em desenvolvimento, com senha interativa:

```bash
cargo run -p ferrugem-web --features server -- \
  bootstrap-admin \
  --username admin \
  --email admin@seudominio.com
```

Opcao de automacao:

```bash
BOOTSTRAP_ADMIN_PASSWORD='senha-super-segura' \
cargo run -p ferrugem-web --features server -- \
  bootstrap-admin \
  --username admin \
  --email admin@seudominio.com
```

Em producao, prefira executar o binario ja construído dentro do container:

```bash
docker compose exec ferrugem-web \
  /app/ferrugem-web bootstrap-admin \
  --username admin \
  --email admin@seudominio.com
```

`ADMIN_BOOTSTRAP_SECRET` autoriza o bootstrap inicial. `BOOTSTRAP_ADMIN_PASSWORD` define a senha do usuario admin apenas para esse comando. Se `BOOTSTRAP_ADMIN_PASSWORD` nao estiver no ambiente, o processo pede a senha de forma interativa.

## Deploy com Caddy

O repositório agora inclui:

- [Dockerfile](/home/caue/presumidos/Dockerfile): build do binario `ferrugem-web`
- [docker-compose.yml](/home/caue/presumidos/docker-compose.yml): origin sem porta publica + Caddy como entrada oficial + Redis interno para rate limit
- [deploy/Caddyfile](/home/caue/presumidos/deploy/Caddyfile): proxy reverso com HTTPS automatico e headers reconstruidos

Desenho da rede:

- Internet -> Caddy `:80/:443` -> `ferrugem-web:8080`
- `ferrugem-web` -> Redis interno para rate limit persistente
- o origin usa apenas `expose`, sem `ports`
- a rede `origin` e interna e dedicada
- o Caddy tambem entra na rede `public` (nao-interna), para ter saida a
  internet e emitir/renovar certificados (Let's Encrypt) e ser o unico
  publicado em `80/443` (tambem `443/udp` para HTTP/3)
- o Caddy tem IP fixo `172.31.0.10` na rede `origin`
- o app confia apenas em `TRUSTED_PROXY_CIDRS=172.31.0.10/32`

Subida recomendada:

```bash
cp .env.example .env
```

Ajuste no `.env` para producao:

- `APP_ENV=production`
- `APP_DOMAIN=seu-dominio.com`
- `SESSION_SECRET` com 32+ caracteres fortes
- `ADMIN_BOOTSTRAP_SECRET` forte
- `COOKIE_SECURE=true`
- `REQUIRE_TRUSTED_PROXY=true`
- `RATE_LIMIT_BACKEND=redis`
- `REDIS_URL=redis://redis:6379`
- `RATE_LIMIT_IDENTITY_SECRET` com 32+ caracteres fortes (proprio, separado do `SESSION_SECRET`)

Depois suba:

```bash
docker compose up --build -d
```

O bootstrap inicial do admin deve ser executado dentro do container do app:

```bash
docker compose exec ferrugem-web \
  /app/ferrugem-web bootstrap-admin \
  --username admin \
  --email admin@seudominio.com
```

O proxy reconstrói estes headers em vez de encaminhar o que veio do cliente:

- `X-Forwarded-For`
- `X-Real-IP`
- `Forwarded`
- `CF-Connecting-IP` e removido

## Backup e Restore

O banco SQLite roda em modo WAL (`PRAGMA journal_mode = WAL`, ver
`ferrugem-web/src/db.rs`) dentro do volume Docker `app_data`
(`/data/bolao.db`). Os scripts abaixo usam uma imagem auxiliar
(`deploy/backup/`, alpine + `sqlite3`) para gerar backups consistentes com a
aplicação rodando, sem depender de instalar nada na VPS.

### Backup manual

```bash
./deploy/backup.sh
```

O que esse comando faz:

- Roda `sqlite3 /data/bolao.db ".backup '...'"` (seguro com WAL, app ligada).
- Salva o arquivo em `./backups/ferrugem-YYYYMMDD-HHMMSS.db`, **fora** do
  volume `app_data`.
- Roda `PRAGMA integrity_check;` no backup gerado; se falhar, o arquivo é
  apagado e o script retorna erro.
- Ajusta o dono do arquivo para o usuário que rodou o script e aplica
  `chmod 600`.
- Remove backups com mais de 14 dias em `./backups/`.

`./backups/` é criado com `chmod 700` e está no `.gitignore` — nunca vai para
o repositório.

### Backup automático (cron)

Na VPS, agende a execução diária (exemplo às 03h):

```cron
0 3 * * * cd /caminho/do/repo && ./deploy/backup.sh >> /var/log/ferrugem-backup.log 2>&1
```

### Cópia externa

Antes de um deploy importante (ou periodicamente), copie `./backups/` para
fora da VPS, por exemplo:

```bash
rsync -av ./backups/ usuario@outro-host:/caminho/de/backups/ferrugem/
```

### Restore em produção

```bash
./deploy/restore.sh backups/ferrugem-20260612-030000.db
```

Esse script:

1. Valida `PRAGMA integrity_check;` do backup escolhido **antes** de tocar em
   produção.
2. Pede confirmação interativa (`sim`).
3. Cria um backup do estado **atual** em `./backups/` antes de sobrescrever
   (backup pré-restore — permite desfazer caso o arquivo restaurado seja o
   errado).
4. Faz `docker compose down`, substitui `bolao.db`/`bolao.db-wal`/
   `bolao.db-shm` pelo backup escolhido e roda `docker compose up -d`.

### Restore testado (ambiente isolado)

Para validar um backup sem tocar em produção:

```bash
./deploy/restore-test.sh backups/ferrugem-20260612-030000.db
```

Esse script cria um volume e um container Docker temporários (não usa
`app_data` nem a rede `origin`), copia o backup para esse volume, e sobe a
aplicação em `http://localhost:18080` com `APP_ENV=development` e
`RATE_LIMIT_BACKEND=memory` (dispensa Redis). Use para confirmar que o login
de admin funciona e que pools/predictions/matches aparecem. Ao encerrar com
`Ctrl+C`, o container e o volume temporários são removidos automaticamente.

## Checklist de fechamento do origin

Critérios de pronto esperados neste layout:

1. `curl http://IP_DA_VPS:8080` falha ou nao conecta.
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
