# Arquitetura do Presumidos

Este documento registra boundaries de código, não um catálogo completo de arquivos. A Fase 19 reorganiza a estrutura sem alterar rotas, contratos HTTP, schema, migrations ou regras de produto.

## Boundaries de backend

- `api.rs` concentra tipos de request/response compartilhados, contexto HTTP, middleware e mapeamento de erros; handlers por capability ficam em `api/{auth_routes,custom_event_routes,pool_routes,prediction_routes,admin_routes}.rs`, e a composição das rotas fica em `api/routes.rs`.
- `events.rs`, `custom_events.rs` e `custom_questions.rs` formam as capabilities de identidade/lifecycle de Event, conteúdo de EventVersion/working revision e questões customizadas.
- `pools.rs` concentra membership, ownership, lifecycle local, leave/delete, convites e projeções de Pool; `pool_access.rs` centraliza o bloqueio e a revelação locais; `pool_scoring.rs` e `pool_tiebreak.rs` concentram, respectivamente, pontuação e desempate por Pool; `prediction_reuse.rs` é a capability explícita de cópia independente entre Pools compatíveis.
- `prediction_items.rs`, `prediction_access.rs` e os módulos de tipo (`multiple_choice.rs`, `numeric.rs`) sustentam validação e acesso de Predictions.
- `matches.rs` trata lifecycle e resultados de partidas; `scoring.rs` calcula o valor dos resultados no Pool. Resultado oficial e scoring permanecem separados.
- `custom_event_manifest/core.rs` concentra modelos, normalização, validação, fingerprint e diff puros; `custom_event_manifest.rs` mantém resolução/persistência e aplica os planos. `event_package.rs` sustenta o empacotamento.
- `assets.rs` é o AssetStore/processamento de variantes; serving HTTP contextual continua em `api.rs` e não altera a política de autorização.
- `auth.rs` é a fachada transversal; `auth_support.rs`, `auth_ops.rs`, `auth_registration.rs` e `auth_login.rs` separam suporte de sessão/crypto/SQL, contas, registro/recuperação e login/sessão. `security.rs` e `context.rs` continuam transversais para CSRF, rate limits e request context.
- `operability.rs`, `config.rs` e `db.rs` formam a camada operacional de configuração, health/readiness, migrations, backup/restore e estado de runtime.

## Frontend

Pages coordenam telas. A fachada compatível `apps/web/src/hooks/queries.ts` reexporta hooks separados por capability em `apps/web/src/hooks/queries/{auth,pools,predictions,admin}.ts`, mantendo query keys, URLs e invalidações. Derivações de filtros do Admin vivem em `apps/web/src/hooks/useAdminMatchFilters.ts`. Capabilities visuais que têm estado e contrato próprios vivem em componentes próximos à feature: `PoolShareModal.tsx`, `PredictionReuseModal.tsx`, `PtBrDateTimeInput.tsx`, `EventBuilderItems.tsx`, `admin/AdminEventsPanel.tsx`, `admin/AdminMatchesPanel.tsx`, `admin/AdminPredictionsPanel.tsx`, `admin/TeamSelect.tsx` e `admin/fixtureValidation.ts` são usados pelas pages sem duplicar lógica de compartilhamento, reuso, edição de perguntas/mídia, gestão de eventos/jogos/palpites, datas localizadas ou validação de fixtures.

## Football

Football é um domínio suportado nativamente pelo Presumidos.

`matches.rs` concentra lifecycle de partidas, horários, resultados oficiais, mata-mata e penalidades. `scoring.rs` mantém as regras de pontuação de football.

O Presumidos não possui integração automática com provider esportivo, poller ou realtime de resultados. Resultados de partidas são administrados pelo fluxo interno e permanecem independentes de serviços externos.

Compatibilidades históricas como `world-cup-2026` e `jogo-*` permanecem somente em migrations, fixtures e testes de regressão quando necessárias para preservar dados históricos.

## Invariantes preservados

- Event identifica o lifecycle; EventVersion publicada é imutável; Pool mantém `event_version_id` autoritativo.
- Prediction pertence ao Pool; reuso é cópia independente e não compartilha mutações.
- EventVersion define o calendário padrão. Um Pool de Event customizado pode fechar seus palpites antecipadamente sem alterar a versão, revelando dados somente entre seus membros; `closed_at` preserva o histórico final do Pool.
- Resultados dizem o que aconteceu; scoring calcula quanto vale no Pool.
- A EventVersion customizada pode sugerir prioridades de desempate, mas a regra efetiva pertence ao Pool: herdar, personalizar ou desativar. Pontuação e desempate congelam juntos em `predictions_closed_at`; a colocação usa critérios de negócio, e o nome só estabiliza a apresentação de empates reais.
- Convites, assets, auth/CSRF, middleware e operações mantêm suas políticas existentes.
