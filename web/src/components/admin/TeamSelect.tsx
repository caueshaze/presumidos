import { formatSelectionLabel, getSelectionGroups, isKnownSelection } from "@/lib/selections";
import { Select } from "@/components/ui/field";

export function TeamSelect({
  value,
  onChange,
  ariaLabel,
}: {
  value: string;
  onChange: (value: string) => void;
  ariaLabel?: string;
}) {
  const groups = getSelectionGroups();
  const unknown = value !== "" && !isKnownSelection(value);
  return (
    <Select value={value} onChange={(e) => onChange(e.target.value)} aria-label={ariaLabel}>
      <option value="">Selecione a seleção</option>
      {unknown && <option value={value}>{formatSelectionLabel(value)}</option>}
      <optgroup label="Seleções">
        {groups.teams.map((selection) => (
          <option key={selection.key} value={selection.name}>
            {formatSelectionLabel(selection.name)}
          </option>
        ))}
      </optgroup>
      {groups.placeholders.length > 0 && (
        <optgroup label="Chaves do mata-mata">
          {groups.placeholders.map((selection) => (
            <option key={selection.key} value={selection.name}>
              {formatSelectionLabel(selection.name)}
            </option>
          ))}
        </optgroup>
      )}
    </Select>
  );
}

