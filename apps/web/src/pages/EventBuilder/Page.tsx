import { ChevronDown } from "lucide-react";
import { PageShell } from "@/components/PageShell";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ErrorBanner, Label, Select } from "@/components/ui/field";
import { AssetUploadControl } from "@/components/AssetUploadControl";
import { EventBuilderItems } from "@/components/EventBuilderItems";
import { PtBrDateTimeInput } from "@/components/PtBrDateTimeInput";
import { useEventBuilder } from "./useEventBuilder";
import { EventPreview } from "./EventPreview";
import { CreateEvent, EventLoading } from "./EntryStates";
import { VersionHistory } from "./VersionHistory";
import { EventPackageExport } from "@/components/EventPackageExport";

export function EventBuilderPage() {
  const { eventId, navigate, isAdmin, user, deleteEvent, draft, setDraft, name, setName, startsAt, setStartsAt, endsAt, setEndsAt, description, setDescription, coverUrl, setCoverUrl, externalUrl, setExternalUrl, title, setTitle, itemKind, setItemKind, decimalPlaces, setDecimalPlaces, unitLabel, setUnitLabel, minValue, setMinValue, maxValue, setMaxValue, minSelections, setMinSelections, maxSelections, setMaxSelections, lockAt, setLockAt, showAddItemForm, setShowAddItemForm, openAddOptionItemId, setOpenAddOptionItemId, labels, setLabels, mediaDrafts, setMediaDrafts, openMediaOptionId, setOpenMediaOptionId, editingOptionId, optionLabelDraft, setOptionLabelDraft, editingItemId, itemTitleDraft, setItemTitleDraft, itemLockDraft, setItemLockDraft, results, setResults, multipleResults, setMultipleResults, error, busy, create, addItem, addOption, action, saveMetadata, startItemEdit, cancelItemEdit, saveItemEdit, startOptionEdit, cancelOptionEdit, saveOptionLabel, saveOptionMedia, deleteOwnedEvent, restoreVersion, load } = useEventBuilder();
  if (!eventId) return <CreateEvent {...{ navigate, name, setName, startsAt, setStartsAt, endsAt, setEndsAt, create, busy, error }} />;
  if (!draft) return <EventLoading {...{ navigate, error }} />;
  // Admins edit published events through the isolated working revision. The
  // public view remains read-only; publication is the explicit commit step.
  const draftOnly = draft.event.status === "draft";
  const editable = draftOnly || isAdmin;
  const metadataEditable = editable || isAdmin;
  const isOwner = draft.event.createdBy === user?.id;
  const hasPools = draft.versions.some((version) => version.poolCount > 0);
  const deletionLabel = draftOnly && !hasPools ? "Excluir rascunho" : "Arquivar evento";
  const mediaEditable = editable || isAdmin || draft.event.createdBy === user?.id;
  const hasInternalAssets = Boolean(
    draft.event.coverAssetId ||
      draft.items.some((item) => item.options.some((option) => option.imageAssetUrl)),
  );
  return (
    <PageShell>
      <Button variant="link" size="sm" onClick={() => navigate("/events")}>← Voltar aos eventos</Button>
      <div className="mt-4 flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0 flex-1">
          <h1 className="text-3xl">{draft.event.name}</h1>
          <p className="text-ink-muted">
            {editable
              ? draftOnly
                ? "Rascunho privado · Escolha única, múltipla escolha ou número"
                : "Revisão de trabalho · publicada somente após confirmação"
              : "Publicado · estrutura imutável"}
          </p>
        </div>
        {editable ? (
          <div className="grid w-full grid-cols-2 gap-2 sm:flex sm:w-auto sm:flex-wrap sm:justify-end">
            <EventPackageExport eventId={draft.event.id} slug={draft.event.name} compact />
            {isOwner && (
              <Button
                variant="outline"
                className="w-full text-danger sm:w-auto"
                disabled={busy || deleteEvent.isPending}
                onClick={() => void deleteOwnedEvent()}
              >
                {deletionLabel}
              </Button>
            )}
            <Button
              className="col-span-2 w-full sm:col-span-1 sm:w-auto"
              disabled={busy}
              onClick={() => action(`/custom/events/${draft.event.id}/publish`)}
            >
              {draftOnly ? "Publicar" : "Publicar revisão"}
            </Button>
          </div>
        ) : (
          <div className="grid w-full grid-cols-2 gap-2 sm:flex sm:w-auto sm:flex-wrap sm:justify-end">
            <EventPackageExport eventId={draft.event.id} slug={draft.event.name} compact />
            <Button className="col-span-2 w-full sm:col-span-1 sm:w-auto" onClick={() => navigate(`/dashboard?eventId=${draft.event.id}`)}>Criar bolão</Button>
            {isOwner && <Button size="sm" variant="outline" className="col-span-2 w-full text-danger sm:col-span-1 sm:w-auto" disabled={busy || deleteEvent.isPending} onClick={() => void deleteOwnedEvent()}>{deletionLabel}</Button>}
          </div>
        )}
      </div>
      {error && (
        <div className="mt-3">
          <ErrorBanner>{error}</ErrorBanner>
        </div>
      )}
      {hasInternalAssets && (
        <p className="mt-3 rounded-lg border border-sky/25 bg-sky/5 px-3 py-2 text-sm text-ink-muted">
          Este evento usa arquivos locais. Para promovê-lo para outro ambiente,
          exporte o pacote completo; o JSON contém apenas as referências por hash.
        </p>
      )}
      <VersionHistory isAdmin={isAdmin} versions={draft.versions} busy={busy} restore={restoreVersion} />
      <Card id="event-images" className="mt-5">
        <div className="flex flex-col gap-3">
          <label>
            Nome do evento
            <Input
              disabled={!metadataEditable}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </label>
          <label>
            Data inicial
            <PtBrDateTimeInput disabled={!editable} value={startsAt} onChange={setStartsAt} />
          </label>
          <label>
            Data final
            <PtBrDateTimeInput disabled={!editable} value={endsAt} onChange={setEndsAt} />
          </label>
          <label>
            Descrição <span className="text-ink-muted">(opcional)</span>
            <textarea disabled={!metadataEditable} value={description} onChange={(e) => setDescription(e.target.value)} maxLength={1200} className="mt-1 min-h-24 w-full rounded-xl border border-mint/25 bg-card/60 px-3 py-2" />
          </label>
          <label>
            Capa do evento
            {draft && (
              <AssetUploadControl
                label="Capa do evento"
                currentUrl={draft.event.coverAssetUrl ?? coverUrl}
                fallbackUrl={draft.event.coverAssetUrl ? coverUrl : undefined}
                uploadPath={`/custom/events/${draft.event.id}/cover`}
                removePath={`/custom/events/${draft.event.id}/cover/remove`}
                disabled={!mediaEditable}
                onChanged={(asset) => setDraft((current) => current ? { ...current, event: { ...current.event, coverAssetId: asset?.assetId ?? null, coverAssetUrl: asset?.url ?? null } } : current)}
              />
            )}
            <details className="mt-3">
              <summary className="cursor-pointer text-sm font-semibold text-mint-dark">{coverUrl ? "Editar URL externa" : "Usar URL externa"}</summary>
              <Input disabled={!metadataEditable} className="mt-2" type="url" placeholder="https://..." value={coverUrl} onChange={(e) => setCoverUrl(e.target.value)} />
            </details>
          </label>
          <label>
            Site oficial <span className="text-ink-muted">(opcional)</span>
            <Input disabled={!metadataEditable} type="url" placeholder="https://..." value={externalUrl} onChange={(e) => setExternalUrl(e.target.value)} />
          </label>
          {metadataEditable && (
            <Button
              variant="secondary"
              disabled={busy}
              onClick={() => void saveMetadata()}
            >
              Salvar informações
            </Button>
          )}
        </div>
      </Card>
      {editable && (
        <Card className="mt-5">
          <button
            type="button"
            className="flex w-full items-center justify-between gap-3 text-left"
            aria-expanded={showAddItemForm}
            onClick={() => setShowAddItemForm((current) => !current)}
          >
            <span>
              <span className="block text-xl font-heading font-semibold">Adicionar pergunta</span>
              <span className="mt-1 block text-sm text-ink-muted">
                Crie uma nova pergunta para este evento
              </span>
            </span>
            <ChevronDown className={`h-5 w-5 shrink-0 text-ink-muted transition-transform ${showAddItemForm ? "rotate-180" : ""}`} />
          </button>
          {showAddItemForm && (
            <form onSubmit={addItem} className="mt-4 flex flex-col gap-3 border-t border-mint/15 pt-4">
            <div>
              <Label htmlFor="item-kind">Tipo da pergunta</Label>
              <Select
                id="item-kind"
                value={itemKind}
                onChange={(e) =>
                  setItemKind(
                    e.target.value as
                      "single_choice" | "multiple_choice" | "numeric",
                  )
                }
              >
                <option value="single_choice">Escolha única</option>
                <option value="multiple_choice">Múltipla escolha</option>
                <option value="numeric">Número</option>
              </Select>
            </div>
            <label>
              Pergunta
              <Input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                required
              />
            </label>
            {itemKind === "multiple_choice" && (
              <>
                <label>
                  Mínimo de escolhas
                  <Input
                    type="number"
                    min="1"
                    value={minSelections}
                    onChange={(e) => setMinSelections(e.target.value)}
                    required
                  />
                </label>
                <label>
                  Máximo de escolhas (opcional)
                  <Input
                    type="number"
                    min="1"
                    value={maxSelections}
                    onChange={(e) => setMaxSelections(e.target.value)}
                  />
                </label>
              </>
            )}
            {itemKind === "numeric" && (
              <>
                <label>
                  Unidade (opcional)
                  <Input
                    value={unitLabel}
                    onChange={(e) => setUnitLabel(e.target.value)}
                  />
                </label>
                <label>
                  Casas decimais
                  <Input
                    type="number"
                    min="0"
                    max="6"
                    value={decimalPlaces}
                    onChange={(e) => setDecimalPlaces(e.target.value)}
                    required
                  />
                </label>
                <label>
                  Valor mínimo (opcional)
                  <Input
                    inputMode="decimal"
                    value={minValue}
                    onChange={(e) => setMinValue(e.target.value)}
                  />
                </label>
                <label>
                  Valor máximo (opcional)
                  <Input
                    inputMode="decimal"
                    value={maxValue}
                    onChange={(e) => setMaxValue(e.target.value)}
                  />
                </label>
              </>
            )}
            <label>
              Fecha palpites em
              <PtBrDateTimeInput value={lockAt} onChange={setLockAt} />
            </label>
            <p className="text-xs text-ink-muted">O resultado aparecerá assim que for definido pelo administrador ou pelo dono do bolão.</p>
            <Button disabled={busy}>Adicionar pergunta</Button>
            </form>
          )}
        </Card>
      )}
      <EventBuilderItems
        draft={draft}
        editable={editable}
        mediaEditable={mediaEditable}
        busy={busy}
        editingItemId={editingItemId}
        itemTitleDraft={itemTitleDraft}
        setItemTitleDraft={setItemTitleDraft}
        itemLockDraft={itemLockDraft}
        setItemLockDraft={setItemLockDraft}
        editingOptionId={editingOptionId}
        optionLabelDraft={optionLabelDraft}
        setOptionLabelDraft={setOptionLabelDraft}
        openMediaOptionId={openMediaOptionId}
        setOpenMediaOptionId={setOpenMediaOptionId}
        mediaDrafts={mediaDrafts}
        setMediaDrafts={setMediaDrafts}
        openAddOptionItemId={openAddOptionItemId}
        setOpenAddOptionItemId={setOpenAddOptionItemId}
        labels={labels}
        setLabels={setLabels}
        results={results}
        setResults={setResults}
        multipleResults={multipleResults}
        setMultipleResults={setMultipleResults}
        action={action}
        load={load}
        addOption={addOption}
        startItemEdit={startItemEdit}
        cancelItemEdit={cancelItemEdit}
        saveItemEdit={saveItemEdit}
        startOptionEdit={startOptionEdit}
        cancelOptionEdit={cancelOptionEdit}
        saveOptionLabel={saveOptionLabel}
        saveOptionMedia={saveOptionMedia}
      />
      <EventPreview items={draft.items} />
    </PageShell>
  );
}
