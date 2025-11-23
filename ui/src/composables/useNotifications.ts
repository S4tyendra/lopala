import { ref } from 'vue'

export type NotifType = 'info' | 'success' | 'warning' | 'error'

export interface Notification {
  id: string
  type: NotifType
  title: string
  body?: string
  icon?: string  // emoji or svg string
  duration: number
}

export const notifications = ref<Notification[]>([])

export function pushNotif(n: Omit<Notification, 'id'>) {
  const id = Math.random().toString(36).substring(7)
  notifications.value.unshift({ ...n, id })

  setTimeout(() => {
    notifications.value = notifications.value.filter(x => x.id !== id)
  }, n.duration)
}

// ─── Convenience shortcuts ────────────────────────────────────────────────────
export const notif = {
  userJoined: (name: string) => pushNotif({
    type: 'success',
    title: `${name} joined`,
    icon: '👤',
    duration: 4000,
  }),
  userLeft: (name: string) => pushNotif({
    type: 'warning',
    title: `${name} left`,
    icon: '👋',
    duration: 3500,
  }),
  wsConnected: () => pushNotif({
    type: 'success',
    title: 'Connected to server',
    icon: '🔗',
    duration: 3000,
  }),
  wsDisconnected: () => pushNotif({
    type: 'error',
    title: 'Disconnected',
    body: 'Reconnecting…',
    icon: '⚡',
    duration: 5000,
  }),
}
