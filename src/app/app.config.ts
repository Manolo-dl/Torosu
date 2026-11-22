import { provideTaiga } from "@taiga-ui/core";
import {
  ApplicationConfig,
  provideBrowserGlobalErrorListeners
} from "@angular/core";
import { provideRouter } from "@angular/router";
import { provideI18n } from "@shared/i18n";

import { routes } from "./app.routes";

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    // A page comes in by itself as it opens (styles.css).
    provideRouter(routes),
    provideTaiga(),
    // After Taiga, whose language it sets.
    provideI18n(),
  ],
};
