# Presumidos — Production Runbook

Este documento descreve a operação da aplicação. Event Package é conteúdo
editorial portátil; não é backup. O backup operacional preserva SQLite e o
AssetStore juntos.

## Configuração e startup

Valide a configuração sem iniciar HTTP:

```bash
/app/ferrugem-web check-config
```

Em `production`, `DATABASE_PATH`, `PRESUMIDOS_ASSET_DIR` e
`PRESUMIDOS_BACKUP_DIR` devem ser absolutos; `SESSION_SECRET` e
`ADMIN_BOOTSTRAP_SECRET` devem ser fortes, diferentes e nunca podem ser valores
de exemplo. Segredos não são incluídos em backups nem nos logs.

O volume persistente deve conter `/data/bolao.db` e `/data/assets`. O diretório
`/backups` deve estar fora do volume de dados quando possível e possuir controle
de acesso adequado: backups contêm dados privados da aplicação.

## Deploy e migrations

O fluxo suportado é:

```text
backup válido → build → app parada → migrate → startup → /health/ready 200
```

Comandos de diagnóstico:

```bash
/app/ferrugem-web migrate --check
/app/ferrugem-web db check
```

Em produção o startup não aplica migrations automaticamente. Migrations são
aplicadas explicitamente por `migrate`; schema pendente, dirty ou com checksum
alterado impede o processo de entrar em readiness.

Rollback não usa down migration automática. Após uma migration incompatível,
retire a versão do tráfego, restaure um backup compatível em manutenção e suba a
versão anterior correspondente.

## Health e sinais

`GET /health/live` responde se o processo HTTP está vivo. `GET /health/ready`
responde 200 apenas quando SQLite/schema, AssetStore e espaço mínimo estão
utilizáveis; falhas críticas respondem 503 sem revelar caminhos, SQL ou secrets.

O limite padrão de espaço livre é 100 MiB e pode ser alterado por
`PRESUMIDOS_MIN_FREE_BYTES`. O endpoint opcional `/internal/metrics` permanece
desabilitado por padrão; quando habilitado, deve ser exposto somente pela rede
de confiança.

No startup são removidos somente resíduos com nomes de staging conhecidos do
AssetStore, backup e restore; diretórios arbitrários nunca entram nessa limpeza.

## Backup

Crie um backup em um diretório dedicado e vazio ou em um diretório de retenção:

```bash
/app/ferrugem-web backup create --output /backups
/app/ferrugem-web backup verify /backups/backup-<timestamp>-<id>
```

Cada backup contém `database.db`, `assets.zip` e `backup.json`. O banco é
copiado por snapshot SQLite (`VACUUM INTO`, seguro com WAL), e o sucesso só é
publicado depois de checksum, `integrity_check`, archive e referências de
assets serem verificados.

Não copie `.env`, chaves TLS, secrets, cookies, tokens ou credenciais para o
backup. A proteção criptográfica deve ser fornecida pelo volume/provider da
infraestrutura, não por criptografia própria da aplicação.

A retenção deve ser feita fora do processo, por cron, systemd timer ou scheduler
da infraestrutura. Exemplo: manter somente backups verificados com mais de uma
cópia fora do host e remover diretórios antigos após confirmar a política de
retenção.

## Restore

Restore é offline/manutenção e exige destino explícito:

```bash
/app/ferrugem-web backup verify /backups/backup-<timestamp>-<id>
/app/ferrugem-web backup restore \
  --input /backups/backup-<timestamp>-<id> \
  --database /data/bolao.db \
  --assets /data/assets \
  --replace
```

Sem `--replace`, destinos existentes não são sobrescritos. O restore valida o
backup, extrai para staging, valida o SQLite e só então troca DB/assets. A troca
usa rename dentro de cada filesystem; DB e assets em volumes distintos não têm
uma transação física única, por isso o procedimento deve permanecer offline.

Após restaurar:

```bash
/app/ferrugem-web db check
curl -fsS http://127.0.0.1:8080/health/ready
```

## Shutdown e incidentes

SIGTERM e SIGINT marcam readiness como indisponível, deixam de aceitar novos
requests no servidor, aguardam requests em andamento e encerram dentro de
`SHUTDOWN_TIMEOUT_SECS` (30 segundos por padrão). O container possui grace
period compatível.

Erros 5xx devolvem apenas uma mensagem segura e `errorId`; logs estruturados
correlacionam `request_id` e `error_id`. Authorization, cookies, passwords,
CSRF, tokens e secrets são redigidos.

Quando readiness retorna 503:

1. rode `db check` e `migrate --check`;
2. verifique logs e espaço dos volumes;
3. confirme que `/data/assets` contém os assets referenciados;
4. não altere o banco manualmente; restaure backup verificado se necessário.

Quando houver disco cheio, preserve o estado atual, libere espaço fora do
SQLite/AssetStore e repita a operação. Uploads usam staging e não devem deixar
referência parcial.

## Checklist de release

- [ ] `check-config` retorna 0 sem imprimir secrets
- [ ] backup criado e `backup verify` retorna 0
- [ ] `migrate --check` foi avaliado
- [ ] migration aplicada com app fora do tráfego
- [ ] `/health/live` retorna 200
- [ ] `/health/ready` retorna 200
- [ ] logs sem falha crítica
- [ ] espaço livre e volume persistente confirmados
- [ ] plano de restore/rollback compatível está disponível
