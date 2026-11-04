import { DOCUMENT, inject, Injectable, signal } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
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

    async use(language: Language) {
        this.current.set(language);
        this.transloco.setActiveLang(language);
        this.document.documentElement.lang = language;
        this.taiga.set(
            language === 'es' ? (await import('@taiga-ui/i18n/languages/spanish')).TUI_SPANISH_LANGUAGE : TUI_ENGLISH_LANGUAGE,
        );
    }
}
