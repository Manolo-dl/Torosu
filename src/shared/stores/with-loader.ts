import { inject, ProviderToken, Signal } from '@angular/core';
import type { ResolveFn } from '@angular/router';
import {
  patchState,
  signalStoreFeature,
  withMethods,
  withState,
  type StateSignals,
  type WritableStateSource,
} from '@ngrx/signals';
import { NotifyService } from '@shared/services';

export interface LoaderOptions<State, Data> {
  /** Reads the data from the backend. */
  read: () => Promise<Data>;
  /** What the store keeps of it. */
  loaded: (data: Data) => Partial<State>;
  /** Notifications once read and when it fails; none if not given. */
  success?: (data: Data) => string;
  failure?: string;
  /**
   * Whether `load()` rejects when the read fails (the default), which makes
   * a resolver stay on the page it was on.
   */
  rethrow?: boolean;
}

interface LoaderState {
  isLoading: boolean;
  isLoaded: boolean;
}

/**
 * State read from the backend once: `initialState` plus `isLoading` and
 * `isLoaded`, with `load()`, `ensureLoaded()` and `reset()`.
 */
export function withLoader<State extends object, Data>(initialState: State, options: LoaderOptions<State, Data>) {
  return signalStoreFeature(
    withState(initialState),
    withState<LoaderState>({ isLoading: false, isLoaded: false }),

    withMethods((store, notify = inject(NotifyService)) => {
      // The store as its two parts: with `State` generic, TypeScript can
      // only check the writes to each part on its own.
      const stored = store as unknown as WritableStateSource<State>;
      const status = store as unknown as WritableStateSource<LoaderState> & StateSignals<LoaderState>;

      // Shared by whoever asks while it is being read (a page and its
      // resolver, or a tool left before it loaded and the next one), so it
      // is read once.
      let loading: Promise<void> | null = null;
      // Bumped by `reset()`: what a read started before it brings is dropped.
      let generation = 0;

      async function read() {
        const started = generation;
        patchState(status, { isLoading: true });

        try {
          const data = await options.read();
          if (started !== generation) return;
          patchState(stored, options.loaded(data));
          patchState(status, { isLoading: false, isLoaded: true });
          if (options.success) notify.success(options.success(data));
        } catch (error) {
          if (started !== generation) return;
          patchState(status, { isLoading: false });
          if (options.failure) notify.error(options.failure);
          if (options.rethrow ?? true) throw error;
        }
      }

      function load(): Promise<void> {
        if (!loading) {
          const current: Promise<void> = read().finally(() => {
            // After a reset, `loading` may be a newer read.
            if (loading === current) loading = null;
          });
          loading = current;
        }
        return loading;
      }

      return {
        load,

        /** Loads it if it is not loaded yet. */
        ensureLoaded(): Promise<void> {
          return status.isLoaded() ? Promise.resolve() : load();
        },

        /** Back to `initialState`, e.g. for another osu! folder. */
        reset() {
          generation++;
          loading = null;
          patchState(stored, initialState);
          patchState(status, { isLoading: false, isLoaded: false });
        },
      };
    }),
  );
}

interface Loader {
  isLoaded: Signal<boolean>;
  load(): Promise<void>;
}

/** Resolver that loads a `withLoader` store before its page opens, saying so. */
export function loaderResolver(store: ProviderToken<Loader>, message: string): ResolveFn<void> {
  return async () => {
    const loader = inject(store);
    if (loader.isLoaded()) return;

    inject(NotifyService).info(message);
    await loader.load();
  };
}
