import { locale } from '@tauri-apps/plugin-os';

/** The languages of the interface, each named in itself. */
export const LANGUAGES = [
    { code: 'en', name: 'English' },
    { code: 'es', name: 'Español' },
] as const;

export type Language = (typeof LANGUAGES)[number]['code'];

/** The one every text is written in first, used where another lacks one. */
export const DEFAULT_LANGUAGE: Language = 'en';

export function isLanguage(code: string | null | undefined): code is Language {
    return LANGUAGES.some(language => language.code === code);
}

/** The system's language if the interface has it (`es-ES` is `es`), English otherwise. */
export async function systemLanguage(): Promise<Language> {
    const tag = (await locale().catch(() => null)) ?? navigator.language ?? '';
    const code = tag.split(/[-_]/)[0].toLowerCase();
    return isLanguage(code) ? code : DEFAULT_LANGUAGE;
}
