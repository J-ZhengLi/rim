export interface CliPayload {
    path: string,
    command: string,
}

export enum ProgressStyle {
    Bytes = 'bytes',
    Len = 'len',
    Spinner = 'spinner',
    Hidden = 'hidden',
}

export interface ProgressPayload {
    message: string,
    style: ProgressStyle,
    length?: number,
}
