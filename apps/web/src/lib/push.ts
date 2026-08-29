import { api } from "@/lib/api";
import type { NotificationPreference, NotificationStatus, WebPushSubscriptionInput } from "@/types";

export const REMINDER_PRESETS = [10, 20, 30] as const;

export interface PushBrowserState {
  serviceWorkerSupported: boolean;
  notificationsSupported: boolean;
  pushManagerSupported: boolean;
  permission: NotificationPermission | "unsupported";
  isStandalone: boolean;
  isProbablyIosBrowser: boolean;
}

function browserState(): PushBrowserState {
  const serviceWorkerSupported = typeof navigator !== "undefined" && "serviceWorker" in navigator;
  const notificationsSupported = typeof window !== "undefined" && "Notification" in window;
  const pushManagerSupported = typeof window !== "undefined" && "PushManager" in window;
  const permission = notificationsSupported ? Notification.permission : "unsupported";
  const isStandalone =
    typeof window !== "undefined" &&
    (window.matchMedia?.("(display-mode: standalone)")?.matches ||
      (typeof navigator !== "undefined" &&
        "standalone" in navigator &&
        Boolean((navigator as Navigator & { standalone?: boolean }).standalone)));
  const isProbablyIosBrowser =
    typeof navigator !== "undefined" &&
    "standalone" in navigator &&
    !isStandalone &&
    !pushManagerSupported;

  return {
    serviceWorkerSupported,
    notificationsSupported,
    pushManagerSupported,
    permission,
    isStandalone: Boolean(isStandalone),
    isProbablyIosBrowser,
  };
}

function ensurePushSupported(state: PushBrowserState) {
  if (!state.serviceWorkerSupported || !state.notificationsSupported || !state.pushManagerSupported) {
    if (state.isProbablyIosBrowser) {
      throw new Error("No iPhone/iPad, adicione o app a tela inicial para ativar notificacoes.");
    }
    throw new Error("Este navegador nao suporta notificacoes web neste fluxo.");
  }
}

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = "=".repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/");
  const rawData = window.atob(base64);
  const outputArray = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; i += 1) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

function applicationServerKey(base64String: string): ArrayBuffer {
  const bytes = urlBase64ToUint8Array(base64String);
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  return buffer;
}

export async function registerServiceWorker() {
  const state = browserState();
  if (!state.serviceWorkerSupported) return null;
  return navigator.serviceWorker.register("/sw.js");
}

async function getReadyRegistration() {
  await registerServiceWorker();
  return navigator.serviceWorker.ready;
}

export async function getCurrentPushSubscription() {
  const state = browserState();
  if (!state.serviceWorkerSupported || !state.pushManagerSupported) return null;
  const registration = await getReadyRegistration();
  return registration.pushManager.getSubscription();
}

function subscriptionToInput(subscription: PushSubscription): WebPushSubscriptionInput {
  const json = subscription.toJSON();
  return {
    endpoint: subscription.endpoint,
    expirationTime: subscription.expirationTime ?? null,
    keys: {
      p256dh: json.keys?.p256dh ?? "",
      auth: json.keys?.auth ?? "",
    },
    userAgent: navigator.userAgent,
    deviceLabel: null,
  };
}

export async function subscribeCurrentBrowser(status: NotificationStatus) {
  const state = browserState();
  ensurePushSupported(state);
  if (!status.webPushEnabled || !status.vapidPublicKey) {
    throw new Error("Notificacoes web nao estao habilitadas neste ambiente.");
  }
  if (state.permission === "denied") {
    throw new Error("As notificacoes estao bloqueadas neste navegador. Libere a permissao e tente novamente.");
  }

  const permission =
    state.permission === "granted" ? "granted" : await Notification.requestPermission();
  if (permission !== "granted") {
    throw new Error("Permissao de notificacao nao concedida.");
  }

  const registration = await getReadyRegistration();
  let subscription = await registration.pushManager.getSubscription();
  if (!subscription) {
    subscription = await registration.pushManager.subscribe({
      userVisibleOnly: true,
      applicationServerKey: applicationServerKey(status.vapidPublicKey),
    });
  }

  await api.post("/notifications/subscriptions", subscriptionToInput(subscription));
  return subscription;
}

export async function unsubscribeCurrentBrowser() {
  const subscription = await getCurrentPushSubscription();
  if (!subscription) return;
  await api.post("/notifications/subscriptions/remove", { endpoint: subscription.endpoint });
  await subscription.unsubscribe();
}

export async function enablePushReminders(
  status: NotificationStatus,
  preference: NotificationPreference,
) {
  await subscribeCurrentBrowser(status);
  await api.post<NotificationPreference>("/notifications/preferences", preference);
}

export function getPushBrowserState() {
  return browserState();
}
