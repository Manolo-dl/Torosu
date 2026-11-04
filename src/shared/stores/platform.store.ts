import { computed } from "@angular/core";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { platform } from '@tauri-apps/plugin-os';
import { invoke } from '@tauri-apps/api/core';


interface PlatformState {
    os: 'windows' | 'linux' | 'unknown';
    /** A Wayland session (Linux), where global shortcuts don't work; `null` until known. */
    wayland: boolean | null;
}
const initialState: PlatformState = {
    os: 'unknown',
    wayland: null,
};
export const PlatformStore = signalStore(
    { providedIn: "root" },
    withState(initialState),

    withComputed((store) => ({
        isWindows: computed(() => store.os() === 'windows'),
        isLinux: computed(() => store.os() === 'linux'),
    })),

    withMethods((store) => ({
        setOs() {
            const os = platform() as 'windows' | 'linux';
            patchState(store, { os });
            invoke<boolean>('is_wayland')
                .then(wayland => patchState(store, { wayland }))
                .catch(e => console.error('is_wayland failed', e));
        }
    })),
);