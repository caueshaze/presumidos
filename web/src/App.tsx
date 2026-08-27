import { lazy, Suspense } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { AuthProvider } from "@/hooks/useAuth";
import { Layout } from "@/components/Layout";
import { AuthGuard } from "@/components/AuthGuard";
import { HomePage } from "@/pages/Home";
import { LoginPage } from "@/pages/Login";
import { RegisterPage } from "@/pages/Register";
import { ForgotPasswordPage } from "@/pages/ForgotPassword";
import { TermsPage } from "@/pages/Terms";
import { PrivacyPage } from "@/pages/Privacy";
import { ContactPage } from "@/pages/Contact";
const DashboardPage = lazy(() => import("@/pages/Dashboard").then((module) => ({ default: module.DashboardPage })));
const PredictionsPage = lazy(() => import("@/pages/Predictions").then((module) => ({ default: module.PredictionsPage })));
const PoolPredictionsPage = lazy(() => import("@/pages/PoolPredictions").then((module) => ({ default: module.PoolPredictionsPage })));
const LeaderboardPage = lazy(() => import("@/pages/Leaderboard").then((module) => ({ default: module.LeaderboardPage })));
const AdminPage = lazy(() => import("@/pages/Admin").then((module) => ({ default: module.AdminPage })));
const ContaPage = lazy(() => import("@/pages/Conta").then((module) => ({ default: module.ContaPage })));
const PoolScoringPage = lazy(() => import("@/pages/PoolScoring").then((module) => ({ default: module.PoolScoringPage })));
const PoolOverviewPage = lazy(() => import("@/pages/PoolOverview").then((module) => ({ default: module.PoolOverviewPage })));
const EventBuilderPage = lazy(() => import("@/pages/EventBuilder").then((module) => ({ default: module.EventBuilderPage })));
const PoolsPage = lazy(() => import("@/pages/Pools").then((module) => ({ default: module.PoolsPage })));
const EventsPage = lazy(() => import("@/pages/Events").then((module) => ({ default: module.EventsPage })));
const PoolInvitePage = lazy(() => import("@/pages/PoolInvite").then((module) => ({ default: module.PoolInvitePage })));

function RouteFallback() {
  return <div className="mx-auto max-w-[1100px] px-5 py-12 text-ink-muted">Carregando...</div>;
}

export function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<HomePage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/register" element={<RegisterPage />} />
            <Route path="/forgot-password" element={<ForgotPasswordPage />} />
            <Route path="/terms" element={<TermsPage />} />
            <Route path="/privacy" element={<PrivacyPage />} />
            <Route path="/contact" element={<ContactPage />} />
            <Route path="/pools/join/:token" element={<PoolInvitePage />} />
            <Route
              path="/events/new"
              element={<AuthGuard><EventBuilderPage /></AuthGuard>}
            />
            <Route path="/events/:eventId" element={<AuthGuard><EventBuilderPage /></AuthGuard>} />
            <Route
              path="/dashboard"
              element={
                <AuthGuard>
                  <DashboardPage />
                </AuthGuard>
              }
            />
            <Route path="/pools" element={<AuthGuard><PoolsPage /></AuthGuard>} />
            <Route path="/events" element={<AuthGuard><EventsPage /></AuthGuard>} />
            <Route
              path="/predictions"
              element={
                <AuthGuard>
                  <PredictionsPage />
                </AuthGuard>
              }
            />
            <Route path="/pools/:poolId/predictions" element={<AuthGuard><PredictionsPage /></AuthGuard>} />
            <Route path="/pools/:poolId" element={<AuthGuard><PoolOverviewPage /></AuthGuard>} />
            <Route path="/pools/:poolId/scoring" element={<AuthGuard><PoolScoringPage /></AuthGuard>} />
            <Route path="/pools/:poolId/leaderboard" element={<AuthGuard><LeaderboardPage /></AuthGuard>} />
            <Route path="/pools/:poolId/members" element={<AuthGuard><PoolPredictionsPage /></AuthGuard>} />
            <Route
              path="/palpites-do-bolao"
              element={
                <AuthGuard>
                  <PoolPredictionsPage />
                </AuthGuard>
              }
            />
            <Route
              path="/leaderboard"
              element={
                <AuthGuard>
                  <LeaderboardPage />
                </AuthGuard>
              }
            />
            <Route
              path="/admin"
              element={
                <AuthGuard>
                  <AdminPage />
                </AuthGuard>
              }
            />
            <Route
              path="/conta"
              element={
                <AuthGuard>
                  <ContaPage />
                </AuthGuard>
              }
            />
            <Route path="*" element={<HomePage />} />
          </Route>
        </Routes>
        </Suspense>
      </AuthProvider>
    </BrowserRouter>
  );
}
