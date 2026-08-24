# Manifestos portáteis de Events

O manifesto é a definição portátil de um Event customizado. Ele não é um
template separado e não contém estado operacional de Pools, usuários ou
predictions.

## Formato

O formato exportado atual é `schemaVersion: 2`; a importação continua aceitando
`schemaVersion: 1` para compatibilidade. A ordem dos arrays `items`, `options` e
`links` é a ordem canônica do evento. O export é JSON UTF-8 indentado, estável,
com newline final, e usa o nome de arquivo `{slug}.json`.

O conteúdo inclui metadata do evento, itens `single_choice`, `multiple_choice`
ou `numeric`, opções, regras numeric/multiple-choice, `lockAt`, `revealAt`,
`imageUrl`, links editoriais e, quando houver upload interno, referências
portáteis `coverAsset`/`imageAsset` no formato:

```json
{"kind":"asset","sha256":"<64 hex>","mediaType":"image/webp"}
```

As referências são hashes do master WebP, nunca IDs internos, caminhos locais ou
base64. O JSON externo continua sendo preservado como fallback quando não há
asset interno.

O conteúdo não inclui IDs internos, usuários, Pools, scoring, predictions,
leaderboard, reações, sessões, resultados oficiais, timestamps internos ou
secrets.

## Promotion DEV → PROD

1. No DEV, crie ou edite o Event no Builder.
2. Na aba Admin → Events, escolha `Exportar JSON`.
3. Leve o arquivo para o ambiente PROD-like.
4. Escolha `Importar JSON` e execute `Validar e visualizar`.
5. Revise o resumo e o diff.
6. Confirme `Aplicar manifesto`.
7. O Event novo nasce como `draft`; revise-o no Builder e publique-o
   explicitamente.

O preview não altera o banco. O apply repete a validação, verifica o fingerprint
da base e executa todas as alterações em uma transação.

## Fingerprints e diff

O pipeline é:

```text
parse → normalize/canonicalize → validate → fingerprint → compare/plan → apply
```

Whitespace, pretty-print e ordem das propriedades JSON não mudam o resultado.
O `baseFingerprint` cobre apenas os campos importáveis do Event; Pools,
predictions e audit logs não invalidam um preview.

Em Events publicados ou finalizados, são seguras as alterações de `name`,
`description`, `coverUrl`, `externalUrl`, imagens e links editoriais das
opções. Slug, external keys, tipos, datas, títulos, labels, ordering, lock/reveal,
numeric config e regras multiple-choice são estruturais e geram `Conflict`.

## CLI

O comando legado continua disponível:

```text
import-custom-event --file evento.json --dry-run
import-custom-event --file evento.json --apply
```

O CLI preserva a política histórica de criar o Event como `active`; a UI Admin
usa a política mais conservadora de criar como `draft`.

A aba Admin de Events exibe status, autoria, contagens de perguntas/opções/pools
e `updatedAt`; esses contadores são apenas uma projeção operacional e não entram
no manifesto nem no fingerprint.

## Smoke recomendado

Use dois bancos descartáveis:

```text
DEV export A
PROD import A → Create → Apply
PROD import A novamente → NoChange

DEV altera somente metadata → export B
PROD preview B → SafeUpdate → Apply

DEV altera título/label/configuração estrutural → export C
PROD preview C → Conflict → zero alterações
```

O VMA 2026 deve preservar 19 itens, 121 opções, metadata e links no round-trip.

Para repetir o smoke de serviço sem depender de bind TCP local, rode:

```text
cargo test --workspace http_tests::contextual_asset_upload_http_smoke_without_tcp -- --exact --test-threads=1
cargo test --workspace http_tests::user_event_builder_asset_pool_flow_works_without_tcp -- --exact --test-threads=1
cargo test --workspace http_tests::admin_package_http_flow_works_without_tcp -- --exact --test-threads=1
cargo test --workspace http_tests::package_promotion_two_sqlite_smoke -- --ignored --exact --nocapture
```

O último teste cria dois SQLite e dois diretórios de assets independentes e
executa Create, NoChange, SafeUpdate, Conflict, fingerprint stale e round-trip
sem copiar banco, transportar SQL ou reusar arquivos entre ambientes. Ele
também repete o fluxo VMA com 19 itens, 121 opções, 4 links e assets internos
em um subset editorial, incluindo SafeUpdate do pacote.

Esses comandos exercitam o serviço e o Router em memória. O smoke visual pela
UI ainda deve ser executado em um host onde o navegador consiga acessar o
servidor local.

## Upload e storage de assets

O Builder oferece upload de capa e de imagem de opção; URL externa fica como
opção avançada. O servidor aceita somente JPEG, PNG e WebP reais, limita o upload
a 10 MB e a imagem a 25 MP, aplica orientação EXIF, remove metadata ao reencodar
para WebP e gera `thumb` (até 320 px), `card` (até 640 px), `cover` (até 1280
px) e `master`, sempre preservando proporção e sem upscale/crop.

Os arquivos ficam no `AssetStore` de filesystem, configurável por
`PRESUMIDOS_ASSET_DIR`. Em Docker, o compose usa `/data/assets`, no mesmo volume
persistent de `/data/bolao.db`. Não existe biblioteca de assets nem S3/MinIO
nesta fase. Assets órfãos podem permanecer no storage; não há garbage collector.
Em produção a configuração exige um caminho absoluto; não use o filesystem
efêmero da camada da imagem do container.

`GET /media/assets/{assetId}/{variant}` serve WebP com cache immutable e ETag
baseado em hash. Assets ligados a Events publicados/finalizados são públicos;
assets de draft exigem a sessão do dono ou de admin. O upload/remove exige sessão,
CSRF e a mesma política editorial do manifesto. O dono ou um administrador pode
trocar/remover a mídia do próprio Event publicado; isso não libera edição de
perguntas, opções, datas ou regras.

Na troca de imagem, os novos bytes são decodificados e gravados em staging antes
de alterar a referência do Event. O promote usa um diretório endereçado pelo
hash; somente depois da validação do arquivo a transação SQLite grava `assets`,
variants e a FK contextual. Se a validação ou o promote falhar, a referência
anterior permanece. Como filesystem e SQLite não compartilham transação, uma
falha posterior aciona compensação sob `BEGIN IMMEDIATE`: hash sem referência,
suas rows e seu diretório são removidos de forma serializada. A limpeza geral de
órfãos não é automática e fica fora desta fase.

Para backup, preserve o banco SQLite e o diretório de assets como um par. O
backup operacional gera `ferrugem-*.db` e `ferrugem-assets-*.tar.gz`; restaurar
somente um deles pode deixar referências sem arquivo ou arquivos sem referência.
Após restore, execute `PRAGMA integrity_check`, confira a presença do volume de
assets e faça um export/import de verificação. O storage deve estar em volume
persistent e incluído no snapshot do host.

## Event Package

`Exportar pacote` gera `{slug}.zip` com exatamente `manifest.json` e, para cada
asset interno referenciado, um master `assets/<sha256>.webp`. Variants nunca são
transportadas: são reconstruídas no ambiente destino. O pacote é UTF-8,
deduplicado por SHA-256 e não contém IDs, usuários, pools, scoring, predictions,
resultados ou secrets.

O import valida zip slip, caminhos absolutos, diretórios, symlinks, entradas
extras/duplicadas, quantidade/tamanho descompactado, UTF-8, WebP real, pixels e
hash do conteúdo. Preview mostra o plano do manifesto e assets existentes/novos;
apply ingere apenas masters ausentes e aplica o mesmo Manifest Service do JSON.
Fingerprint obsoleto é rejeitado antes de ingerir assets; se uma aplicação falhar
depois da ingestão, a compensação remove os assets novos que não ficaram
referenciados. O Event criado pela UI nasce `draft`; uma segunda importação
idêntica é `NoChange` e não duplica Event, items, opções ou links.

## Compatibilidade e operação

JSON v1 sem referências internas continua aceito e é normalizado para o formato
canônico v2 no preview/apply/export. O CLI legado preserva sua criação `active`;
a UI Admin usa `draft`. Não há botão force. `name`, descrição, URLs, imagens e
links são editoriais seguros após publicação; slug, identidade/ordem, tipos,
datas operacionais, títulos/labels, lock/reveal e configurações de regras são
estruturais e geram `Conflict`.

Eventos administrativos e comunitários continuam sendo a mesma entidade
`Event`; autoria, autorização e riqueza editorial são políticas, não tabelas
paralelas. Pools, Predictions, scoring e resultados não são alterados pelo
gerenciamento de apresentação.
