import { EnvironmentProviders, inject, isDevMode, Provider } from '@angular/core';
import { provideTransloco } from '@jsverse/transloco';
import { TUI_LANGUAGE } from '@taiga-ui/i18n/tokens';
import { DEFAULT_LANGUAGE, LANGUAGES } from './languages';
import { LanguageService } from './language.service';
import { TranslationLoader } from './translation.loader';

/** Translation of the interface; after `provideTaiga()`, whose language it replaces. */
export function provideI18n(): (Provider | EnvironmentProviders)[] {
    return [
        provideTransloco({
            config: {
                availableLangs: LANGUAGES.map(language => language.code),
                defaultLang: DEFAULT_LANGUAGE,
                // A text not translated yet shows in English.
                fallbackLang: DEFAULT_LANGUAGE,
                missingHandler: { useFallbackTranslation: true, logMissingKey: isDevMode() },
                reRenderOnLangChange: true,
                prodMode: !isDevMode(),
            },
            loader: TranslationLoader,
        }),
        { provide: TUI_LANGUAGE, useFactory: () => inject(LanguageService).taiga.asReadonly() },
    ];
}
