export interface SelectionCatalogEntry {
  key: string;
  name: string;
  emoji: string;
  kind: "team" | "placeholder";
  aliases?: string[];
}

interface BaseSelectionDefinition {
  key: string;
  name: string;
  emoji: string;
  aliases?: string[];
}

const NEUTRAL_PLACEHOLDER_EMOJI = "◻";

const REAL_SELECTIONS: BaseSelectionDefinition[] = [
  { key: "south-africa", name: "África do Sul", emoji: "🇿🇦" },
  { key: "germany", name: "Alemanha", emoji: "🇩🇪" },
  { key: "argentina", name: "Argentina", emoji: "🇦🇷" },
  { key: "algeria", name: "Argélia", emoji: "🇩🇿" },
  { key: "saudi-arabia", name: "Arábia Saudita", emoji: "🇸🇦" },
  { key: "australia", name: "Austrália", emoji: "🇦🇺" },
  { key: "austria", name: "Áustria", emoji: "🇦🇹" },
  { key: "brazil", name: "Brasil", emoji: "🇧🇷" },
  { key: "belgium", name: "Bélgica", emoji: "🇧🇪" },
  { key: "bosnia-and-herzegovina", name: "Bósnia e Herzegovina", emoji: "🇧🇦" },
  { key: "cape-verde", name: "Cabo Verde", emoji: "🇨🇻" },
  { key: "canada", name: "Canadá", emoji: "🇨🇦" },
  { key: "colombia", name: "Colômbia", emoji: "🇨🇴" },
  { key: "south-korea", name: "Coreia do Sul", emoji: "🇰🇷", aliases: ["Coréia do Sul"] },
  { key: "ivory-coast", name: "Costa do Marfim", emoji: "🇨🇮" },
  { key: "croatia", name: "Croácia", emoji: "🇭🇷" },
  { key: "curacao", name: "Curaçao", emoji: "🇨🇼" },
  { key: "egypt", name: "Egito", emoji: "🇪🇬" },
  { key: "ecuador", name: "Equador", emoji: "🇪🇨" },
  { key: "scotland", name: "Escócia", emoji: "🏴󠁧󠁢󠁳󠁣󠁴󠁿" },
  { key: "spain", name: "Espanha", emoji: "🇪🇸" },
  { key: "united-states", name: "Estados Unidos", emoji: "🇺🇸", aliases: ["EUA"] },
  { key: "france", name: "França", emoji: "🇫🇷" },
  { key: "ghana", name: "Gana", emoji: "🇬🇭" },
  { key: "haiti", name: "Haiti", emoji: "🇭🇹" },
  { key: "england", name: "Inglaterra", emoji: "🏴󠁧󠁢󠁥󠁮󠁧󠁿" },
  { key: "iraq", name: "Iraque", emoji: "🇮🇶" },
  { key: "iran", name: "Irã", emoji: "🇮🇷" },
  { key: "japan", name: "Japão", emoji: "🇯🇵" },
  { key: "jordan", name: "Jordânia", emoji: "🇯🇴" },
  { key: "morocco", name: "Marrocos", emoji: "🇲🇦" },
  { key: "mexico", name: "México", emoji: "🇲🇽" },
  { key: "norway", name: "Noruega", emoji: "🇳🇴" },
  { key: "new-zealand", name: "Nova Zelândia", emoji: "🇳🇿" },
  { key: "panama", name: "Panamá", emoji: "🇵🇦" },
  { key: "paraguay", name: "Paraguai", emoji: "🇵🇾" },
  { key: "netherlands", name: "Países Baixos", emoji: "🇳🇱", aliases: ["Paises Baixos", "Holanda"] },
  { key: "portugal", name: "Portugal", emoji: "🇵🇹" },
  { key: "qatar", name: "Qatar", emoji: "🇶🇦" },
  { key: "dr-congo", name: "RD Congo", emoji: "🇨🇩", aliases: ["Republica Democratica do Congo"] },
  { key: "senegal", name: "Senegal", emoji: "🇸🇳" },
  { key: "sweden", name: "Suécia", emoji: "🇸🇪" },
  { key: "switzerland", name: "Suíça", emoji: "🇨🇭" },
  { key: "czechia", name: "Tchéquia", emoji: "🇨🇿", aliases: ["Republica Tcheca", "República Tcheca"] },
  { key: "tunisia", name: "Tunísia", emoji: "🇹🇳" },
  { key: "turkey", name: "Turquia", emoji: "🇹🇷" },
  { key: "uruguay", name: "Uruguai", emoji: "🇺🇾" },
  { key: "uzbekistan", name: "Uzbequistão", emoji: "🇺🇿" },
];

const PLACEHOLDER_SELECTIONS: SelectionCatalogEntry[] = [
  ..."ABCDEFGHIJKL".split("").flatMap((group) => [
    { key: `group-${group}-first`, name: `1º Grupo ${group}`, emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" as const },
    { key: `group-${group}-second`, name: `2º Grupo ${group}`, emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" as const },
  ]),
  { key: "best-third-a-b-c-d-f", name: "3º Grupo A/B/C/D/F", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-a-e-h-i-j", name: "3º Grupo A/E/H/I/J", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-b-e-f-i-j", name: "3º Grupo B/E/F/I/J", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-c-d-f-g-h", name: "3º Grupo C/D/F/G/H", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-c-e-f-h-i", name: "3º Grupo C/E/F/H/I", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-d-e-i-j-l", name: "3º Grupo D/E/I/J/L", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-e-f-g-i-j", name: "3º Grupo E/F/G/I/J", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "best-third-e-h-i-j-k", name: "3º Grupo E/H/I/J/K", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  ...Array.from({ length: 30 }, (_, index) => index + 73).map((matchNumber) => ({
    key: `winner-match-${matchNumber}`,
    name: `Vencedor Jogo ${matchNumber}`,
    emoji: NEUTRAL_PLACEHOLDER_EMOJI,
    kind: "placeholder" as const,
  })),
  { key: "loser-match-101", name: "Perdedor Jogo 101", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
  { key: "loser-match-102", name: "Perdedor Jogo 102", emoji: NEUTRAL_PLACEHOLDER_EMOJI, kind: "placeholder" },
];

export const SELECTION_CATALOG: SelectionCatalogEntry[] = [
  ...REAL_SELECTIONS.map((entry) => ({ ...entry, kind: "team" as const })),
  ...PLACEHOLDER_SELECTIONS,
];

const normalizedNameToEntry = new Map<string, SelectionCatalogEntry>();

for (const entry of SELECTION_CATALOG) {
  normalizedNameToEntry.set(normalizeSelectionKey(entry.name), entry);
  for (const alias of entry.aliases ?? []) {
    normalizedNameToEntry.set(normalizeSelectionKey(alias), entry);
  }
}

function normalizeSelectionKey(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/\s+/g, " ")
    .trim();
}

export function getSelectionCatalogEntry(name: string): SelectionCatalogEntry | null {
  return normalizedNameToEntry.get(normalizeSelectionKey(name)) ?? null;
}

export function isKnownSelection(name: string): boolean {
  return getSelectionCatalogEntry(name) !== null;
}

export function formatSelectionLabel(name: string): string {
  const entry = getSelectionCatalogEntry(name);
  if (!entry) return `${NEUTRAL_PLACEHOLDER_EMOJI} ${name}`;
  return `${entry.emoji} ${entry.name}`;
}

export function getSelectionGroups() {
  return {
    teams: SELECTION_CATALOG.filter((entry) => entry.kind === "team"),
    placeholders: SELECTION_CATALOG.filter((entry) => entry.kind === "placeholder"),
  };
}
