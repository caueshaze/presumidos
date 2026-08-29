# Fase 20 — remoção do provider de futebol

## Removido

- Integração externa de futebol (`football.rs`, `football/domain.rs` e
  `football/integration.rs`), incluindo cliente HTTP, retries, backoff e parser.
- Poller de startup, CLI `sync-fixtures`, configuração `FOOTBALL_*` e a
  dependência Rust exclusiva do provider.
- Rotas administrativas de mapeamento/consulta de fixture e sincronização.
- UI de IDs externos, sincronização, placar parcial e ranking provisório.
- Overlay de scoring baseado em placar ao vivo.

Foram removidas aproximadamente 3.000 linhas líquidas, 5 rotas e uma tarefa de
background. `reqwest` permanece apenas como dependência de desenvolvimento dos
testes HTTP; deixou de integrar o feature de servidor.

## Preservado

- Partidas, palpites, placar oficial, pênaltis, mata-mata, scoring e ranking.
- Administração protegida de partidas, horário, resultado manual e auditoria.
- EventVersion/Working Revision, Pools e histórico persistido.
- Schema e migrations históricos: não houve migration destrutiva; colunas antigas
  permanecem como compatibilidade física sem consumidores no runtime.

## Copa legacy

`world-cup-2026` e IDs `jogo-*` permanecem somente em migrations, fixtures e
testes de regressão histórica. Não há branch de provider nem comportamento de
runtime ligado a esses identificadores.

## Rotas

Antes: 130 rotas. Depois: 125 rotas.

- `POST /admin/matches/{id}/fixture` — mapeamento de provider removido.
- `POST /admin/fixtures/check` — consulta ao provider removida.
- `GET /admin/sync/status` — status do provider removido.
- `POST /admin/sync/run-now` — sincronização manual removida.
- `POST /admin/sync/backfill` — backfill externo removido.

## Validação

- `cargo fmt --all -- --check` e `git diff --check` passaram.
- `cargo check -p ferrugem-web` passou.
- `cargo run -q -p ferrugem-web -- check-config` passou com as antigas variáveis
  de provider ausentes.
- `cargo test --workspace -- --test-threads=1` executou a suíte ativa de 125
  testes sem falhas; o teste de regressão da finalização manual também foi
  executado isoladamente após a última remoção.
- `npm --prefix web test -- --run`: 11 arquivos e 39 testes passaram.
- `npm --prefix web run lint` e `npm --prefix web run build` passaram.
- Busca de termos ESPN/provider/poller/sync no código ativo sem ocorrências.
