import { provideTaiga } from "@taiga-ui/core";
import {
  ApplicationConfig,
  inject,
  provideBrowserGlobalErrorListeners
} from "@angular/core";
import { provideRouter, withViewTransitions } from "@angular/router";
import { AppearanceStore } from "@shared/stores";
import { provideI18n } from "@shared/i18n";

import { routes } from "./app.routes";

export const appConfig: ApplicationConfig = {
  providers: [
    provideBrowserGlobalErrorListeners(),
    // Pages change in a View Transition where the engine has them: only the
    // page's panel moves (styles.css), and a tool's icon flies from its card
    // on the home to the page's header. Not with reduced motion, nor with the
    // transitions turned off in the settings.
    provideRouter(
      routes,
      withViewTransitions({
        skipInitialTransition: true,
        onViewTransitionCreated: ({ transition }) => {
          const appearance = inject(AppearanceStore);
          const reduced = appearance.reducedMotion() || matchMedia('(prefers-reduced-motion: reduce)').matches;
          if (reduced || !appearance.transitions()) {
            transition.skipTransition();
          }
        },
      }),
    ),
    provideTaiga(),
    // After Taiga, whose language it sets.
    provideI18n(),
  ],
};
