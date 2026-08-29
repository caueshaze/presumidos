import type { AdminMatchesPanelProps } from "./admin-matches/types";
import { KnockoutManagement } from "./admin-matches/KnockoutManagement";
import { MatchList } from "./admin-matches/MatchList";
import { MatchEditor } from "./admin-matches/MatchEditor";

export function AdminMatchesPanel(props: AdminMatchesPanelProps) {
  return (
    <div className="mt-6 space-y-5">
      <KnockoutManagement {...props} />
      <div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr] [&>*]:min-w-0">
        <MatchList {...props} />
        <MatchEditor {...props} />
      </div>
    </div>
  );
}
