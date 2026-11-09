import { Injectable } from '@angular/core';
import { Translation, TranslocoLoader } from '@jsverse/transloco';

/**
 * The texts of each language, bundled with the app: each in a chunk of its
 * own, loaded the first time it is used.
 */
@Injectable({ providedIn: 'root' })
export class TranslationLoader implements TranslocoLoader {
    async getTranslation(language: string): Promise<Translation> {
        switch (language) {
            case 'es':
                return (await import('./translations/es.json')).default;
            case 'fr':
                return (await import('./translations/fr.json')).default;
            case 'zh':
                return (await import('./translations/zh.json')).default;
            default:
                return (await import('./translations/en.json')).default;
        }
    }
}
