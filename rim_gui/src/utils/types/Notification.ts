export interface NotificationAction {
    label: string,
    icon?: string,
    command: [string, string?],
}

export interface Notification {
    title: string,
    content: string,
    actions: NotificationAction[],
}
