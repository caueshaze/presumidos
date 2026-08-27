import { useMemo } from "react";
import { isKnockout } from "@/lib/utils";
import { formatSelectionLabel } from "@/lib/selections";
import type { AdminMatchRecord } from "@/types";

export type AdminMatchFilters = {
  type: string;
  phase: string;
  groupName: string;
  date: string;
  status: string;
  origin: string;
  team: string;
};

export const emptyAdminMatchFilters: AdminMatchFilters = {
  type: "",
  phase: "",
  groupName: "",
  date: "",
  status: "",
  origin: "",
  team: "",
};

export function useAdminMatchFilters(
  filteredMatches: AdminMatchRecord[] | undefined,
  allMatches: AdminMatchRecord[] | undefined,
  filters: AdminMatchFilters,
) {
  const knockoutMatches = useMemo(
    () => (allMatches ?? []).filter((item) => isKnockout(item.matchRecord.phase)),
    [allMatches],
  );

  const phaseOptions = useMemo(() => {
    const phases = new Set<string>();
    for (const item of allMatches ?? []) {
      if (item.matchRecord.phase) phases.add(item.matchRecord.phase);
    }
    return Array.from(phases).sort();
  }, [allMatches]);

  const groupOptions = useMemo(() => {
    const groups = new Set<string>();
    for (const item of allMatches ?? []) {
      if (item.matchRecord.groupName) groups.add(item.matchRecord.groupName);
    }
    return Array.from(groups).sort();
  }, [allMatches]);

  const visibleMatches = useMemo(() => {
    const term = filters.team.trim().toLowerCase();
    return (filteredMatches ?? []).filter((item) => {
      const knockout = isKnockout(item.matchRecord.phase);
      if (filters.type === "group" && knockout) return false;
      if (filters.type === "knockout" && !knockout) return false;
      if (!term) return true;
      const home = formatSelectionLabel(item.matchRecord.homeTeam).toLowerCase();
      const away = formatSelectionLabel(item.matchRecord.awayTeam).toLowerCase();
      return home.includes(term) || away.includes(term);
    });
  }, [filteredMatches, filters.team, filters.type]);

  const hasActiveFilters = Object.values(filters).some((value) => value !== "");

  return { knockoutMatches, phaseOptions, groupOptions, visibleMatches, hasActiveFilters };
}
