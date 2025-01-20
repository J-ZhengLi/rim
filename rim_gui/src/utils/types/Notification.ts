export interface NotificationAction {
    label: string,
    icon?: string,
    command: RustFunction,
}

export interface RustFunction {
    name: string,
    args: [[string, string]],
    ret_id?: string,
}

export interface Notification {
    title: string,
    content: string,
    actions: NotificationAction[],
}
