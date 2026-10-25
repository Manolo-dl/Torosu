import { computed } from "@angular/core";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";

interface NavStoreState {
    mode: 'user' | 'dev';
}

const initialState: NavStoreState = {
    mode: 'user'
};

export const NavStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store) => ({
        isUserMode: computed(() => store.mode() === 'user'),
        isDevMode: computed(() => store.mode() === 'dev' ? 'Dev Mode' : 'User Mode'),
    })),

    withMethods((store) => ({
        toggleMode: () => {
            const currentMode = store.mode();
            const newMode = currentMode === 'user' ? 'dev' : 'user';
            patchState(store, { mode: newMode });
        },

        /** Back to the tools, e.g. when developer mode is turned off. */
        userMode: () => patchState(store, { mode: 'user' }),
    })),
)