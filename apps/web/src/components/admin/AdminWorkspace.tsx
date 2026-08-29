import { Navigate, useNavigate } from "react-router-dom";
import { useAuth } from "@/hooks/useAuth";
import { useAdminWorkspace } from "./useAdminWorkspace";
import { AdminWorkspaceView } from "./AdminWorkspaceView";

export function AdminWorkspace() {
  const { isAdmin, loading } = useAuth();
  const navigate = useNavigate();
  const workspace = useAdminWorkspace({ navigate });

  if (!loading && !isAdmin) return <Navigate to="/" replace />;

  return <AdminWorkspaceView workspace={workspace} />;
}
