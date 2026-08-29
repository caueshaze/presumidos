import { useCallback, useEffect, useMemo, useState } from "react";
import { useNotificationStatus, useUpdateNotificationPreference } from "@/hooks/queries";
import {
  enablePushReminders,
  getCurrentPushSubscription,
  getPushBrowserState,
  REMINDER_PRESETS,
  subscribeCurrentBrowser,
  unsubscribeCurrentBrowser,
  type PushBrowserState,
} from "@/lib/push";
import type { NotificationPreference } from "@/types";

export function usePushReminders() {
  const status = useNotificationStatus();
  const updatePreference = useUpdateNotificationPreference();
  const [browserState, setBrowserState] = useState<PushBrowserState>(getPushBrowserState);
  const [currentSubscription, setCurrentSubscription] = useState<PushSubscription | null>(null);
  const [actionPending, setActionPending] = useState(false);
  const [actionError, setActionError] = useState("");
  const [actionMessage, setActionMessage] = useState("");

  const refreshLocalState = useCallback(async () => {
    setBrowserState(getPushBrowserState());
    const subscription = await getCurrentPushSubscription();
    setCurrentSubscription(subscription);
  }, []);

  useEffect(() => {
    void refreshLocalState();
  }, [refreshLocalState]);

  useEffect(() => {
    if (!actionMessage) return;
    const timer = window.setTimeout(() => setActionMessage(""), 4000);
    return () => window.clearTimeout(timer);
  }, [actionMessage]);

  const preference = status.data?.preference ??
    ({ enabled: false, leadTimeMinutes: 20, reactionEnabled: true } as NotificationPreference);
  const currentDeviceSubscribed = !!currentSubscription;
  const supportsPushFlow =
    browserState.serviceWorkerSupported &&
    browserState.notificationsSupported &&
    browserState.pushManagerSupported;

  const runAction = useCallback(
    async (fn: () => Promise<void>, successMessage: string) => {
      setActionPending(true);
      setActionError("");
      setActionMessage("");
      try {
        await fn();
        setActionMessage(successMessage);
      } catch (error) {
        setActionError(error instanceof Error ? error.message : "Falha ao configurar notificacoes.");
      } finally {
        setActionPending(false);
        await status.refetch();
        await refreshLocalState();
      }
    },
    [refreshLocalState, status],
  );

  const enableForCurrentDevice = useCallback(
    async (leadTimeMinutes: NotificationPreference["leadTimeMinutes"]) =>
      runAction(async () => {
        if (!status.data) throw new Error("Estado de notificacoes ainda nao carregado.");
        await enablePushReminders(status.data, {
          enabled: true,
          leadTimeMinutes,
          reactionEnabled: preference.reactionEnabled,
        });
      }, "Notificacoes ativadas neste dispositivo."),
    [preference.reactionEnabled, runAction, status.data],
  );

  const subscribeThisDeviceOnly = useCallback(
    async () =>
      runAction(async () => {
        if (!status.data) throw new Error("Estado de notificacoes ainda nao carregado.");
        await subscribeCurrentBrowser(status.data);
      }, "Dispositivo conectado para receber notificacoes."),
    [runAction, status.data],
  );

  const disableCurrentDevice = useCallback(
    async () =>
      runAction(async () => {
        await unsubscribeCurrentBrowser();
      }, "Notificacoes desativadas neste dispositivo."),
    [runAction],
  );

  const updateAccountPreference = useCallback(
    async (next: NotificationPreference) =>
      runAction(async () => {
        await updatePreference.mutateAsync(next);
      }, "Preferencia de lembrete atualizada."),
    [runAction, updatePreference],
  );

  return useMemo(
    () => ({
      status,
      preference,
      browserState,
      currentDeviceSubscribed,
      supportsPushFlow,
      actionPending: actionPending || updatePreference.isPending || status.isFetching,
      actionError,
      actionMessage,
      reminderPresets: REMINDER_PRESETS,
      enableForCurrentDevice,
      subscribeThisDeviceOnly,
      disableCurrentDevice,
      updateAccountPreference,
    }),
    [
      actionError,
      actionMessage,
      actionPending,
      browserState,
      currentDeviceSubscribed,
      disableCurrentDevice,
      enableForCurrentDevice,
      preference,
      status,
      subscribeThisDeviceOnly,
      supportsPushFlow,
      updateAccountPreference,
      updatePreference.isPending,
    ],
  );
}
