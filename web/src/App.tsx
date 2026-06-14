import { BrowserRouter, Routes, Route } from "react-router-dom";
import { AuthProvider } from "@/hooks/useAuth";
import { Layout } from "@/components/Layout";
import { AuthGuard } from "@/components/AuthGuard";
import { HomePage } from "@/pages/Home";
import { LoginPage } from "@/pages/Login";
import { RegisterPage } from "@/pages/Register";
import { ForgotPasswordPage } from "@/pages/ForgotPassword";
import { DashboardPage } from "@/pages/Dashboard";
import { PredictionsPage } from "@/pages/Predictions";
import { PoolPredictionsPage } from "@/pages/PoolPredictions";
import { LeaderboardPage } from "@/pages/Leaderboard";
import { AdminPage } from "@/pages/Admin";
import { ContaPage } from "@/pages/Conta";
import { TermsPage } from "@/pages/Terms";
import { PrivacyPage } from "@/pages/Privacy";
import { ContactPage } from "@/pages/Contact";

export function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route path="/" element={<HomePage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/register" element={<RegisterPage />} />
            <Route path="/forgot-password" element={<ForgotPasswordPage />} />
            <Route path="/terms" element={<TermsPage />} />
            <Route path="/privacy" element={<PrivacyPage />} />
            <Route path="/contact" element={<ContactPage />} />
            <Route
              path="/dashboard"
              element={
                <AuthGuard>
                  <DashboardPage />
                </AuthGuard>
              }
            />
            <Route
              path="/predictions"
              element={
                <AuthGuard>
                  <PredictionsPage />
                </AuthGuard>
              }
            />
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
      </AuthProvider>
    </BrowserRouter>
  );
}
