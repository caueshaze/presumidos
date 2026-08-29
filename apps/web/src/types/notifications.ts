export interface NotificationPreference {
  enabled: boolean;
  leadTimeMinutes: 10 | 20 | 30;
  reactionEnabled: boolean;
}
export interface WebPushSubscriptionKeys {
  p256dh: string;
  auth: string;
}
export interface WebPushSubscriptionInput {
  endpoint: string;
  expirationTime: number | null;
  keys: WebPushSubscriptionKeys;
  userAgent?: string | null;
  deviceLabel?: string | null;
}
export interface NotificationStatus {
  webPushEnabled: boolean;
  vapidPublicKey: string | null;
  preference: NotificationPreference;
  activeSubscriptionCount: number;
}
