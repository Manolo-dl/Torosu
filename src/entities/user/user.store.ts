import { User } from "./user.model";
import { patchState, signalStore, withHooks, withMethods, withState } from "@ngrx/signals";
import { invoke } from '@tauri-apps/api/core';

interface UserState {
    user: User | null;
}

const initialState: UserState = {
    user: null,
};

export const UserStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withMethods((store) => ({
        updateUser(partial: Partial<User>) {
            const current = store.user();
            const updated = current ? { ...current, ...partial } : partial as User;
            patchState(store, { user: updated });
            if ('osuSession' in partial) {
                invoke('set_osu_session', { session: partial.osuSession ?? null }).catch(() => {});
            }
        },

        clearSession() {
            patchState(store, { user: null });
            invoke('set_osu_session', { session: null }).catch(() => {});
        },
    })),

    withHooks({
        onInit(store) {
            invoke<string | null>('get_osu_session')
                .then(session => {
                    if (session) patchState(store, { user: { osuSession: session } });
                })
                .catch(() => {});
        }
    })
);
