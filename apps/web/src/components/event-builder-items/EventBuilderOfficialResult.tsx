import { Check, Trophy } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/field";
import type { Item } from "./types";
export function EventBuilderOfficialResult({ item, state }: { item: Item; state: any }) { const { action, results, setResults, multipleResults, setMultipleResults, editable } = state; return <>                {!editable && item.kind === "single_choice" && (
                  <div className="mt-5 rounded-xl border border-sky/25 bg-sky/5 p-4">
                    <div className="flex items-start gap-3">
                      <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-sky/15 text-sky-dark">
                        <Trophy className="h-4 w-4" />
                      </span>
                      <div>
                        <p className="font-semibold text-ink">Resultado oficial</p>
                        <p className="mt-0.5 text-xs text-ink-muted">Selecione o vencedor desta pergunta para fechar a apuração.</p>
                      </div>
                    </div>
                    <div className="mt-3 flex flex-col gap-2 sm:flex-row sm:items-center">
                    <Select
                      aria-label={`Resultado oficial: ${item.title}`}
                      value={results[item.id] ?? item.correctOptionId ?? ""}
                      onChange={(e) =>
                        setResults((v: Record<string, string>) => ({ ...v, [item.id]: e.target.value }))
                      }
                    >
                      <option value="">Selecione o vencedor</option>
                      {item.options.map((o) => (
                        <option key={o.id} value={o.id}>
                          {o.label}
                        </option>
                      ))}
                    </Select>
                    <Button
                      size="sm"
                      className="w-full sm:w-auto"
                      disabled={!(results[item.id] ?? item.correctOptionId)}
                      onClick={() =>
                        action(`/admin/custom/questions/${item.id}/result`, {
                          optionId: results[item.id] ?? item.correctOptionId,
                        })
                      }
                    >
                      <Check className="h-4 w-4" />
                      Salvar resultado
                    </Button>
                    </div>
                  </div>
                )}
                {!editable && item.kind === "multiple_choice" && (
                  <div className="mt-3 space-y-2">
                    <p className="text-sm font-semibold">Resultado oficial</p>
                    {item.options.map((option) => {
                      const selected = multipleResults[item.id] ?? [];
                      return (
                        <label
                          key={option.id}
                          className="flex items-center gap-2"
                        >
                          <input
                            type="checkbox"
                            checked={selected.includes(option.id)}
                            onChange={() =>
                              setMultipleResults((all: Record<string, string[]>) => ({
                                ...all,
                                [item.id]: selected.includes(option.id)
                                  ? selected.filter((id: string) => id !== option.id)
                                  : [...selected, option.id],
                              }))
                            }
                          />
                          {option.label}
                        </label>
                      );
                    })}
                    <Button
                      size="sm"
                      disabled={
                        (multipleResults[item.id]?.length ?? 0) <
                        (item.minSelections ?? 1)
                      }
                      onClick={() =>
                        action(
                          `/admin/custom/multiple-choice/${item.id}/result`,
                          { optionIds: multipleResults[item.id] ?? [] },
                        )
                      }
                    >
                      Salvar resultado
                    </Button>
                  </div>
                )}
            {item.kind === "numeric" && !editable && (
              <div className="mt-3 flex gap-2">
                <Input
                  inputMode="decimal"
                  aria-label={`Resultado oficial: ${item.title}`}
                  value={results[item.id] ?? item.resultValue ?? ""}
                  onChange={(e) =>
                    setResults((v: Record<string, string>) => ({ ...v, [item.id]: e.target.value }))
                  }
                />
                <Button
                  size="sm"
                  disabled={!(results[item.id] ?? item.resultValue)}
                  onClick={() =>
                    action(`/admin/custom/numeric/${item.id}/result`, {
                      value: results[item.id] ?? item.resultValue,
                    })
                  }
                >
                  Salvar resultado
                </Button>
              </div>
            )}</>; }
