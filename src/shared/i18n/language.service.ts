import { DOCUMENT, inject, Injectable, signal } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { TUI_ENGLISH_LANGUAGE } from '@taiga-ui/i18n/languages/english';
import type { TuiLanguage } from '@taiga-ui/i18n/types';
import { DEFAULT_LANGUAGE, Language } from './languages';

/** The language of the interface: the app's texts (Transloco) and Taiga's own. */
@Injectable({ providedIn: 'root' })
export class LanguageService {
    private readonly transloco = inject(TranslocoService);
    private readonly document = inject(DOCUMENT);

    readonly current = signal<Language>(DEFAULT_LANGUAGE);
    /** Taiga's texts (see `TUI_LANGUAGE` in `provideI18n`). */
    readonly taiga = signal<TuiLanguage>(TUI_ENGLISH_LANGUAGE);

    private chosen!: () => void;
    /**
     * Resolves once the language saved in the settings is in use, its texts
     * loaded. Messages shown at startup wait for it: before, `translate` would
     * give the key, or English.
     */
    readonly ready = new Promise<void>(resolve => (this.chosen = resolve));

    async use(language: Language) {
        this.current.set(language);
        this.transloco.setActiveLang(language);
        // `load` shares the request `setActiveLang` started.
        await firstValueFrom(this.transloco.load(language)).catch(() => undefined);
        this.chosen();
        this.document.documentElement.lang = language;
        this.taiga.set(await taigaLanguage(language));
    }
}

/** Taiga's texts in `language`: English is bundled, the others load when chosen. */
async function taigaLanguage(language: Language): Promise<TuiLanguage> {
    switch (language) {
        case 'es':
            return (await import('@taiga-ui/i18n/languages/spanish')).TUI_SPANISH_LANGUAGE;
        case 'fr':
            return (await import('@taiga-ui/i18n/languages/french')).TUI_FRENCH_LANGUAGE;
        case 'zh':
            return (await import('@taiga-ui/i18n/languages/chinese')).TUI_CHINESE_LANGUAGE;
        default:
            return TUI_ENGLISH_LANGUAGE;
    }
}
