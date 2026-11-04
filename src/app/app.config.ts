import { provideTaiga, tuiNotificationOptionsProvider } from "@taiga-ui/core";
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
    provideRouter(routes),
    provideTaiga(),
    // After Taiga, whose language it sets.
    provideI18n(),
    // Defaults for every TuiNotificationService.open() call in the app
    tuiNotificationOptionsProvider({ autoClose: 4000, closable: true }),
  ],
};
