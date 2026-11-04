import { Injectable } from '@angular/core';
import { Translation, TranslocoLoader } from '@jsverse/transloco';

/**
 * The texts of each language, bundled with the app: each in a chunk of its
 * own, loaded the first time it is used.
 */
@Injectable({ providedIn: 'root' })
export class TranslationLoader implements TranslocoLoader {
    async getTranslation(language: string): Promise<Translation> {
        const texts = language === 'es' ? await import('./translations/es.json') : await import('./translations/en.json');
        return texts.default;
    }
}
