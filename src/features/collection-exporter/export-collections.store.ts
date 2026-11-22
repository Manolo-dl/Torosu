import { computed, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { patchState, signalStore, withComputed, withMethods, withState } from '@ngrx/signals';
import { CollectionStore } from '@entities/collection';
import { isUncheckedSet, OsuDbStore, setsOfDiffs } from '@entities/osu-db';
import { OsuBeatmapSet } from '@entities/osu-db/osu-db.model';
import { errorText } from '@shared/i18n';
import { BeatmapsetExportService, NotifyService } from '@shared/services';
import { beatmapsetUrl } from '@shared/utils';

/**
 * `md5`: each map of a collection by its MD5, all of them (also those not
 * installed), for the importer: the same diffs come back. `links`: a link
 * per set, for the downloader or the website; only sets that are on it.
 */
export type CollectionFormat = 'md5' | 'links';

/** A collection of collection.db, as it can be exported. */
export interface ExportRow {
    name: string;
    /** Its maps, each once. */
    md5s: string[];
    /** Its sets in the library, with the diffs it has of each. */
    sets: OsuBeatmapSet[];
    /** Its sets on the website, as links go. */
    links: number[];
    /** Of `md5s`, those not in the library: no set to link to. */
    notInstalled: number;
    /** Its sets in the library that are not on the website: no link to give. */
    unsubmitted: OsuBeatmapSet[];
    selected: boolean;
}

interface ExportCollectionsState {
    /** Collections picked, by name. */
    selected: string[];
    /** What the list is filtered by. */
    query: string;
    format: CollectionFormat;
}

const initialState: ExportCollectionsState = {
    selected: [],
    query: '',
    format: 'links',
};

/** What makes a name unfit for a file, replaced in the file's default name. */
const UNSAFE_FILE_CHARS = /[\\/:*?"<>|]/g;

export const ExportCollectionsStore = signalStore(
    { providedIn: 'root' },
    withState(initialState),

    withComputed((store, collectionStore = inject(CollectionStore), osuDb = inject(OsuDbStore)) => {
        const rows = computed<ExportRow[]>(() => {
            const setsByMd5 = osuDb.beatmapSetsByMd5();
            const submittedByMd5 = osuDb.submittedSetsByMd5();
            const selected = new Set(store.selected());
            return collectionStore.collections().map(collection => {
                const md5s = [...new Set(collection.md5s.map(md5 => md5.toLowerCase()))];
                const sets = setsOfDiffs(md5s, setsByMd5);
                const linked = md5s.filter(md5 => submittedByMd5.has(md5));
                return {
                    name: collection.name,
                    md5s,
                    sets,
                    links: [...new Set(linked.map(md5 => submittedByMd5.get(md5)!.beatmapsetId))],
                    notInstalled: md5s.filter(md5 => !setsByMd5.has(md5)).length,
                    unsubmitted: sets.filter(set => !set.diffs.some(diff => submittedByMd5.has(diff.md5))),
                    selected: md5s.length > 0 && selected.has(collection.name),
                };
            });
        });

        const selectedRows = computed(() => rows().filter(row => row.selected));

        const visibleRows = computed(() => {
            const query = store.query().trim().toLowerCase();
            return query ? rows().filter(row => row.name.toLowerCase().includes(query)) : rows();
        });

        // What the importer reads: a block per collection, `- name` and a map per line.
        const text = computed(() => {
            const links = store.format() === 'links';
            return selectedRows()
                .map(row => ({ name: row.name, lines: links ? row.links.map(beatmapsetUrl) : row.md5s }))
                .filter(section => section.lines.length > 0)
                .map(section => [`- ${section.name}`, ...section.lines].join('\n'))
                .join('\n\n');
        });

        return {
            rows,
            visibleRows,
            selectedRows,
            /** Collections with maps, picked or not. */
            exportable: computed(() => rows().filter(row => row.md5s.length > 0).length),
            maps: computed(() => selectedRows().reduce((sum, row) => sum + row.md5s.length, 0)),
            links: computed(() => selectedRows().reduce((sum, row) => sum + row.links.length, 0)),
            /** Maps picked that are not in the library: links can't carry them. */
            notInstalled: computed(() => selectedRows().reduce((sum, row) => sum + row.notInstalled, 0)),
            /** Sets picked that are not on the website, each once: links can't carry them. */
            unsubmitted: computed(() => new Set(selectedRows().flatMap(row => row.unsubmitted.map(set => set.folderName))).size),
            /** Sets linked that osu! never checked online: they may not be on the website. */
            uncheckedLinks: computed(() => {
                const submittedByMd5 = osuDb.submittedSetsByMd5();
                const unchecked = new Set<number>();
                for (const row of selectedRows()) {
                    for (const md5 of row.md5s) {
                        const set = submittedByMd5.get(md5);
                        if (set && isUncheckedSet(set)) unchecked.add(set.beatmapsetId);
                    }
                }
                return unchecked.size;
            }),
            text,
            lines: computed(() => (text() ? text().split('\n').length : 0)),
        };
    }),

    withMethods((store, exporter = inject(BeatmapsetExportService), notify = inject(NotifyService), transloco = inject(TranslocoService)) => ({
        toggle(name: string) {
            const selected = store.selected();
            patchState(store, { selected: selected.includes(name) ? selected.filter(n => n !== name) : [...selected, name] });
        },

        /** Every collection shown (the search's) picked, or none of them if they all were. */
        toggleAll() {
            const shown = store.visibleRows().filter(row => row.md5s.length > 0).map(row => row.name);
            const allPicked = shown.every(name => store.selected().includes(name));
            patchState(store, {
                selected: allPicked
                    ? store.selected().filter(name => !shown.includes(name))
                    : [...new Set([...store.selected(), ...shown])],
            });
        },

        setQuery: (query: string) => patchState(store, { query }),
        setFormat: (format: CollectionFormat) => patchState(store, { format }),

        async save() {
            const rows = store.selectedRows();
            const name = rows.length === 1 ? rows[0].name.replace(UNSAFE_FILE_CHARS, '_') : 'collections';
            try {
                if (await exporter.saveToFile(store.text(), `${name}.txt`)) {
                    notify.success(transloco.translate('collections.export.saved'));
                }
            } catch (error) {
                notify.error(transloco.translate('collections.export.failed', { error: errorText(transloco, error) }));
            }
        },

        async copy() {
            const text = store.text();
            try {
                await navigator.clipboard.writeText(text);
            } catch {
                // Where the clipboard API is not allowed: the old way.
                const area = document.createElement('textarea');
                area.value = text;
                area.style.position = 'fixed';
                area.style.opacity = '0';
                document.body.append(area);
                area.select();
                const copied = document.execCommand('copy');
                area.remove();
                if (!copied) {
                    notify.error(transloco.translate('collections.export.copyFailed'));
                    return;
                }
            }
            notify.success(transloco.translate('collections.export.copied'));
        },
    })),
);
