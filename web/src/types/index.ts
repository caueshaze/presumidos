// Tipos espelhando os models Rust (camelCase via serde rename_all).

export interface UserPublic {
  id: string;
  username: string;
  email: string;
  isAdmin: boolean;
}

export interface AuthResult {
  user: UserPublic;
  token: string;
  csrfToken: string;
}

export interface SessionState {
  user: UserPublic | null;
  csrfToken: string;
}

export interface PoolSummary {
  id: string;
  name: string;
  inviteCode: string;
  memberCount: number;
}

export interface MatchRecord {
  id: string;
  homeTeam: string;
  awayTeam: string;
  kickoff: string;
  groupName: string | null;
  phase: string | null;
  homeScore: number | null;
  awayScore: number | null;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
  finished: boolean;
}

export interface PredictionRecord {
  matchId: string;
  homeScore: number;
  awayScore: number;
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHomeScore: number | null;
  penaltyAwayScore: number | null;
}

export interface KnockoutEntry {
  qualifier: string | null;
  wentToPenalties: boolean;
  penaltyHome: number | null;
  penaltyAway: number | null;
}

export interface LeaderboardEntry {
  username: string;
  points: number;
}
