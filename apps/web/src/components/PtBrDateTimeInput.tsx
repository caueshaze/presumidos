import { useEffect, useState } from "react";
import { Input } from "@/components/ui/input";

export function localDateTimeValue(value: string): string {
  if (!value) return "";
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value.slice(0, 16);
  const pad = (part: number) => String(part).padStart(2, "0");
  return `${parsed.getFullYear()}-${pad(parsed.getMonth() + 1)}-${pad(parsed.getDate())}T${pad(parsed.getHours())}:${pad(parsed.getMinutes())}`;
}

export function formatPtBrDate(value: string): string {
  const local = localDateTimeValue(value);
  if (!/^\d{4}-\d{2}-\d{2}/.test(local)) return "";
  return `${local.slice(8, 10)}/${local.slice(5, 7)}/${local.slice(0, 4)}`;
}

export function formatPtBrDateTime(value: string): string {
  const local = localDateTimeValue(value);
  return local ? `${formatPtBrDate(local)} às ${local.slice(11, 16)}` : "";
}

export function combinePtBrDateTime(dateText: string, time: string): string | null {
  const match = /^(\d{2})\/(\d{2})\/(\d{4})$/.exec(dateText);
  if (!match || !/^\d{2}:\d{2}$/.test(time)) return null;
  const [, day, month, year] = match;
  const candidate = `${year}-${month}-${day}T${time}`;
  const parsed = new Date(candidate);
  return Number.isNaN(parsed.getTime()) || parsed.getFullYear() !== Number(year) || parsed.getMonth() + 1 !== Number(month) || parsed.getDate() !== Number(day)
    ? null
    : candidate;
}

export function toIsoDateTime(value: string): string {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toISOString();
}

export function PtBrDateTimeInput({ value, onChange, disabled }: { value: string; onChange: (value: string) => void; disabled?: boolean }) {
  const [dateText, setDateText] = useState(formatPtBrDate(value));
  const [time, setTime] = useState(localDateTimeValue(value).slice(11, 16));

  useEffect(() => {
    setDateText(formatPtBrDate(value));
    setTime(localDateTimeValue(value).slice(11, 16));
  }, [value]);

  const updateDate = (raw: string) => {
    const digits = raw.replace(/\D/g, "").slice(0, 8);
    const next = digits.length > 4 ? `${digits.slice(0, 2)}/${digits.slice(2, 4)}/${digits.slice(4)}` : digits.length > 2 ? `${digits.slice(0, 2)}/${digits.slice(2)}` : digits;
    setDateText(next);
    const combined = combinePtBrDateTime(next, time);
    if (combined) onChange(combined);
  };

  const updateTime = (next: string) => {
    const digits = next.replace(/\D/g, "").slice(0, 4);
    const formatted = digits.length > 2 ? `${digits.slice(0, 2)}:${digits.slice(2)}` : digits;
    setTime(formatted);
    const combined = combinePtBrDateTime(dateText, formatted);
    if (combined) onChange(combined);
  };

  return <div className="grid grid-cols-[1fr_8rem] gap-2"><Input type="text" inputMode="numeric" placeholder="dd/mm/aaaa" value={dateText} onChange={(event) => updateDate(event.target.value)} disabled={disabled} aria-label="Data" maxLength={10} /><Input type="text" inputMode="numeric" placeholder="HH:mm" value={time} onChange={(event) => updateTime(event.target.value)} disabled={disabled} aria-label="Hora" maxLength={5} /></div>;
}
