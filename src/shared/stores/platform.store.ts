import { computed } from "@angular/core";
import { patchState, signalStore, withComputed, withMethods, withState } from "@ngrx/signals";
import { platform } from '@tauri-apps/plugin-os';


interface PlatformState {
    os: 'windows' | 'linux' | 'unknown';
}
const initialState: PlatformState = {
    os: 'unknown'
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
        }
    })),
);