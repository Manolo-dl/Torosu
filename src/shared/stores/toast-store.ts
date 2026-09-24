import { patchState, signalStore, withMethods, withState } from "@ngrx/signals";

export interface Toast {
    id: string;
    type: 'success' | 'error' | 'info' | 'warning';
    message: string;
    duration: number;
}

interface ToastStoreState {
    toasts: Toast[];
}

const initialState: ToastStoreState = {
    toasts: [],
};

export const ToastStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withMethods((store) => ({
        show(type: Toast['type'], message: string, duration: number = 4000) {
            const id = crypto.randomUUID();
            const toast: Toast = { id, type, message, duration };

            patchState(store, { toasts: [...store.toasts(), toast] });

            setTimeout(() => {
                patchState(store, { toasts: store.toasts().filter(t => t.id !== id) });
            }, duration);
        },

        dismiss(id: string) {
            patchState(store, { toasts: store.toasts().filter(t => t.id !== id) });
        }
    })),
)