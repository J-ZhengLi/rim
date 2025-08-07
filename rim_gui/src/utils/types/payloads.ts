export interface CliPayload {
    path: string,
    command: string,
}

export enum ProgressKind {
    Bytes = 'bytes',
    Len = 'len',
    Spinner = 'spinner',
    Hidden = 'hidden',
}

export interface ProgressPayload {
    message: string,
    style: ProgressKind,
    length?: number,
}

export interface ToolkitUpdatePayload {
    version: string,
    data?: string,
}
