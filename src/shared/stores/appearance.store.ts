import { DOCUMENT, effect, inject } from '@angular/core';
import { patchState, signalStore, withHooks, withMethods, withState } from '@ngrx/signals';
import { TUI_DARK_MODE } from '@taiga-ui/core';
import { VIEW_TRANSITIONS } from '../utils/view-transitions';

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
    /** The logo and the glow behind the pages move with that music. */
    musicReactive: boolean;
    /** Animated backgrounds: triangles, specks of light, the modes' orbit. */
    effects: boolean;
    /** `low`: fewer of them, drawn at 30 frames a second and at fewer pixels. */
    quality: EffectsQuality;
    /** Pages and the theme change in an animated transition. */
    transitions: boolean;
    /** The beatmap set cards show a cover, made from each map's background. */
    covers: boolean;
}

export const EFFECTS_QUALITIES = ['high', 'low'] as const;
export type EffectsQuality = (typeof EFFECTS_QUALITIES)[number];

const initialState: AppearanceState = {
    palette: 'sakura',
    font: 'outfit',
    dark: true,
    reducedMotion: false,
    sidebarCollapsed: false,
    visualizer: true,
    musicReactive: true,
    effects: true,
    quality: 'high',
    transitions: true,
    covers: true,
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
            musicReactive: value.musicReactive !== false,
            effects: value.effects !== false,
            quality: EFFECTS_QUALITIES.includes(value.quality as EffectsQuality) ? (value.quality as EffectsQuality) : 'high',
            transitions: value.transitions !== false,
            covers: value.covers !== false,
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
        setMusicReactive: (musicReactive: boolean) => patchState(store, { musicReactive }),
        setEffects: (effects: boolean) => patchState(store, { effects }),
        setQuality: (quality: EffectsQuality) => patchState(store, { quality }),
        setTransitions: (transitions: boolean) => patchState(store, { transitions }),
        setCovers: (covers: boolean) => patchState(store, { covers }),
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
                    musicReactive: store.musicReactive(),
                    effects: store.effects(),
                    quality: store.quality(),
                    transitions: store.transitions(),
                    covers: store.covers(),
                };
                const root = document.documentElement;
                root.dataset['palette'] = state.palette;
                root.dataset['font'] = state.font;
                root.dataset['theme'] = state.dark ? 'dark' : 'light';
                root.dataset['motion'] = state.reducedMotion ? 'reduced' : 'full';
                root.dataset['effects'] = state.effects ? 'on' : 'off';
                root.dataset['quality'] = state.quality;
                root.dataset['transitions'] = state.transitions ? 'on' : 'off';
                root.dataset['viewTransitions'] = VIEW_TRANSITIONS ? 'on' : 'off';
                taigaDark.set(state.dark);
                localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
            });
        },
    }),
);
