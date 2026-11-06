/** The languages of the interface, each named in itself. */
export const LANGUAGES = [
    { code: 'en', name: 'English' },
    { code: 'es', name: 'Español' },
    { code: 'fr', name: 'Français' },
] as const;

export type Language = (typeof LANGUAGES)[number]['code'];

/** The one every text is written in first: used where another lacks one, and when none is chosen. */
export const DEFAULT_LANGUAGE: Language = 'en';

export function isLanguage(code: string | null | undefined): code is Language {
    return LANGUAGES.some(language => language.code === code);
}
