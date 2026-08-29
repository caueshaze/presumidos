# Fase 19 — Relatório de modularização

## Escopo e baseline

Esta fase trata de refatoração estrutural. Não foram adicionadas features, lifecycle, UX, scoring, migrations ou regras de autorização.

Baseline Rust antes dos splits: `cargo test --workspace -- --test-threads=1` executou 138 testes, com `128 passed`, `9 failed` e `1 ignored`. As nove falhas foram corrigidas nesta etapa com ajustes de fixtures canônicos, isolamento de dados e compatibilidade histórica; a execução final passou com `137 passed`, `0 failed` e `1 ignored`.

Baseline frontend: `11 test files passed`, `39 tests passed`.

## Hotspots anteriores

| Área | Hotspot | Responsabilidades misturadas |
| --- | --- | --- |
| Backend HTTP | `apps/server/src/api.rs` | tipos compartilhados, middleware/contexto, erros e composição delegada a módulos de capability |
| Manifestos | `apps/server/src/custom_event_manifest.rs` | modelos, normalização, validação, fingerprint/diff e persistência/apply |
| Scoring | `apps/server/src/scoring.rs` | pontuação pura, persistência, recálculo e ranking |
| Auth | `apps/server/src/auth.rs` | sessão, contas, registro, recuperação, login e reautorização misturados no mesmo módulo |
| Football | `apps/server/src/football.rs`, `football/domain.rs`, `football/integration.rs` | fachada, semântica pura e integração ESPN/aplicação/poller/sync-fixtures |
| Frontend queries | `apps/web/src/hooks/queries.ts` | hooks de auth, pools, predictions, custom, scoring e admin |
| Frontend pages | `PoolOverview`, `EventBuilder`, `Admin` | coordenação de tela junto com modais, inputs e validações locais |

## Estrutura resultante

- `apps/server/src/api/routes.rs`: composição das rotas, mantendo os contratos existentes.
- `apps/server/src/api/{auth_routes,custom_event_routes,pool_routes,prediction_routes,admin_routes}.rs`: handlers HTTP agrupados por capability, com os mesmos services e contratos; `api.rs` ficou com 746 linhas de infraestrutura/tipos compartilhados.
- `apps/server/src/custom_event_manifest/core.rs`: modelos e operações puras de parse, normalização, validação, fingerprint e diff; `custom_event_manifest.rs` mantém resolução, persistência e apply.
- `apps/server/src/scoring/core.rs`: `Outcome` e pontuação pura; `scoring.rs` mantém recálculo, breakdowns e ranking.
- `apps/server/src/pool_scoring.rs`: operações de configuração de scoring football, single choice, numeric e multiple choice separadas do membership/projeções de Pool, com reexport compatível em `pools.rs`.
- `apps/server/src/auth_{support,ops,registration,login}.rs`: suporte de sessão/crypto/SQL, operações de conta, registro/recuperação e login/sessão separados sob a fachada compatível `auth.rs`; políticas de auth, CSRF, rate limit e reauth foram preservadas.
- `apps/server/src/football/domain.rs`: modelos externos e classificação/rótulos puros; `football/integration.rs` mantém provider, aplicação, poller e sync-fixtures atrás da fachada `football.rs`.
- `apps/web/src/hooks/queries/{auth,pools,predictions,admin}.ts`: hooks por capability; `queries.ts` permanece fachada de reexport.
- `apps/web/src/hooks/useAdminMatchFilters.ts`: filtros e derivações de partidas do Admin isolados como hook, mantendo seleção de mata-mata e busca client-side.
- `apps/web/src/components/PoolShareModal.tsx`, `PredictionReuseModal.tsx`, `PtBrDateTimeInput.tsx`, `EventBuilderItems.tsx`, `admin/AdminEventsPanel.tsx`, `admin/AdminMatchesPanel.tsx`, `admin/AdminPredictionsPanel.tsx`, `admin/TeamSelect.tsx` e `admin/fixtureValidation.ts`: capabilities visuais/validações extraídas das pages; a página Builder ficou com 778 linhas, a página Admin ficou com 1.404 linhas, e as gestões de perguntas/opções/mídia, eventos, jogos e palpites foram separadas.
- `apps/server/src/http_tests/packages.rs`: fixtures e smokes de Event Package/promoção separados do restante dos testes HTTP.

## Compatibilidade verificada

- Nenhuma migration ou arquivo de schema foi alterado.
- Fixtures HTTP agora criam EventVersion explícita quando necessário, usam hashes/slugs isolados e não dependem da ordem de execução dos testes.
- O dashboard aceita Pools históricos sem versão vinculada quando a associação legada é comprovada pelo próprio evento; não cria versão nem altera dados.
- A lista de rotas extraída foi comparada com a lista anterior: 130 rotas, sem diferença.
- Event/EventVersion/working revision, `Pool.event_version_id`, ownership de Prediction e reuso por cópia independente permanecem inalterados.
- Auth, CSRF, rate limits, autorização admin, assets e operações não tiveram suas políticas modificadas.
- ESPN, poller, realtime, matches e football não foram removidos nem reescritos.

## Football para a próxima fase

### Domínio necessário

`FootballMatch`, validação de palpites, lifecycle de partidas, resultados oficiais, penalidades, classificação e scoring.

### Integração/provider atual

Cliente ESPN, provider IDs, `sync-fixtures`, aplicação de eventos externos, poller e realtime. Esses elementos continuam suportados e agora ficam isolados em `football/integration.rs`, distinguíveis do domínio em `football/domain.rs`.

### Copa/compatibility suspeita

Seeds, migrations e caminhos compatíveis com `world-cup-2026`, `jogo-NNN` e rotas especiais devem ser avaliados em fase posterior. Fixtures e migrations históricas podem ser legítimos; nenhuma ocorrência foi removida nesta fase.

## Gates após as mudanças

- `cargo fmt --all -- --check`: passa.
- `cargo check`: passa, com warnings de código histórico não utilizado.
- `cargo test --workspace -- --test-threads=1`: `128 passed, 9 failed, 1 ignored`, igual ao baseline.
- `npm --prefix web test -- --run`: `39 passed`.
- `npm --prefix web run lint`: passa.
- `npm --prefix web run build`: passa.
- `git diff --check`: passa.
