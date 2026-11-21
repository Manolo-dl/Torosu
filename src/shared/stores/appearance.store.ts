import { DOCUMENT, effect, inject } from '@angular/core';
import { patchState, signalStore, withHooks, withMethods, withState } from '@ngrx/signals';
import { TUI_DARK_MODE } from '@taiga-ui/core';

/** The palettes of shared/styles/theme.css. */
export const PALETTES = ['sakura', 'nebula', 'aurora', 'ember', 'mono'] as const;
export type Palette = (typeof PALETTES)[number];

/** The bundled fonts. */
export const FONTS = ['outfit', 'nunito', 'inter', 'grotesk'] as const;
export type Font = (typeof FONTS)[number];

interface AppearanceState {
    palette: Palette;
    font: Font;
    dark: boolean;
    /** Fewer animations, besides the system's own setting. */
    reducedMotion: boolean;
    /** The sidebar shows only its icons. */
    sidebarCollapsed: boolean;
    /** Bars in the title bar following the music osu! plays. */
    visualizer: boolean;
}

const initialState: AppearanceState = {
    palette: 'sakura',
    font: 'outfit',
    dark: true,
    reducedMotion: false,
    sidebarCollapsed: false,
    visualizer: true,
};

/** Where it is kept: it is the window's look, not a setting of the tools. */
const STORAGE_KEY = 'torosu.appearance';

function saved(): AppearanceState {
    try {
        const value = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}') as Partial<AppearanceState>;
        return {
            palette: PALETTES.includes(value.palette as Palette) ? (value.palette as Palette) : initialState.palette,
            font: FONTS.includes(value.font as Font) ? (value.font as Font) : initialState.font,
            dark: typeof value.dark === 'boolean' ? value.dark : initialState.dark,
            reducedMotion: value.reducedMotion === true,
            sidebarCollapsed: value.sidebarCollapsed === true,
            visualizer: value.visualizer !== false,
        };
    } catch {
        return initialState;
    }
}

/**
 * The look of the app: palette, font, theme and motion, as attributes of
 * <html> that shared/styles/theme.css reads. Taiga's dark mode follows it.
 */
export const AppearanceStore = signalStore(
    { providedIn: 'root' },
    withState(saved),

    withMethods(store => ({
        setPalette: (palette: Palette) => patchState(store, { palette }),
        setFont: (font: Font) => patchState(store, { font }),
        setDark: (dark: boolean) => patchState(store, { dark }),
        toggleDark: () => patchState(store, { dark: !store.dark() }),
        setReducedMotion: (reducedMotion: boolean) => patchState(store, { reducedMotion }),
        toggleSidebar: () => patchState(store, { sidebarCollapsed: !store.sidebarCollapsed() }),
        setVisualizer: (visualizer: boolean) => patchState(store, { visualizer }),
    })),

    withHooks({
        onInit(store, document = inject(DOCUMENT), taigaDark = inject(TUI_DARK_MODE)) {
            effect(() => {
                const state: AppearanceState = {
                    palette: store.palette(),
                    font: store.font(),
                    dark: store.dark(),
                    reducedMotion: store.reducedMotion(),
                    sidebarCollapsed: store.sidebarCollapsed(),
                    visualizer: store.visualizer(),
                };
                const root = document.documentElement;
                root.dataset['palette'] = state.palette;
                root.dataset['font'] = state.font;
                root.dataset['theme'] = state.dark ? 'dark' : 'light';
                root.dataset['motion'] = state.reducedMotion ? 'reduced' : 'full';
                taigaDark.set(state.dark);
                localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
            });
        },
    }),
);
