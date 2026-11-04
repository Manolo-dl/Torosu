import { TranslocoService } from '@jsverse/transloco';

/** How a command fails: the backend's `Error` (src-tauri/src/error.rs). */
export interface CommandError {
    code: string;
    params: Record<string, unknown>;
    /** In English, as the logs say it. */
    message: string;
}

export function isCommandError(value: unknown): value is CommandError {
    const error = value as CommandError;
    return typeof error?.code === 'string' && typeof error.message === 'string';
}

/**
 * The text of `error` in the language in use: a command's from its code
 * (`errors.<code>`), with its cause translated too; its English message
 * without a text for it. Anything else as it is.
 */
export function errorText(transloco: TranslocoService, error: unknown): string {
    if (!isCommandError(error)) return error instanceof Error ? error.message : String(error);

    const params = Object.fromEntries(
        Object.entries(error.params ?? {}).map(([name, value]) => [
            name,
            isCommandError(value)
                ? errorText(transloco, value)
                // The task a convert failed in, by the name the trainer shows.
                : name === 'task'
                  ? transloco.translate(`trainer.tasks.${value}`)
                  : value,
        ]),
    );
    const key = `errors.${error.code}`;
    const text = transloco.translate(key, params);
    return text === key ? error.message : text;
}
