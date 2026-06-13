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
import { LeaderboardPage } from "@/pages/Leaderboard";

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
              path="/leaderboard"
              element={
                <AuthGuard>
                  <LeaderboardPage />
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
