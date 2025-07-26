export interface CliPayload {
    path: string,
    command: string,
}

export interface ProgressPayload {
    message: string,
    length?: number,
}
