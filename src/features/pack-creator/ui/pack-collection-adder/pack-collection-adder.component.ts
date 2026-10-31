import { Component, computed, inject, ChangeDetectionStrategy } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { OsuDbStore } from '@entities/osu-db';
import { PackStore } from '../../pack.store';
import { OsuCollection } from '@entities/collection';
import { TuiButton, TuiLoader, TuiScrollbar, TuiTitle } from '@taiga-ui/core';
import { TuiBadge } from '@taiga-ui/kit';
import { TuiCardLarge } from '@taiga-ui/layout';

interface CollectionEntry {
    collection: OsuCollection;
    available: number;
    added: number;
}

@Component({
    selector: 'app-pack-collection-adder',
    imports: [TuiBadge, TuiButton, TuiCardLarge, TuiLoader, TuiScrollbar, TuiTitle],
    templateUrl: './pack-collection-adder.component.html',
    styleUrl: './pack-collection-adder.component.less',
    changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PackCollectionAdderComponent {
    readonly collectionStore = inject(CollectionStore);
    readonly packStore = inject(PackStore);
    readonly osuDb = inject(OsuDbStore);

    entries = computed<CollectionEntry[]>(() => {
        const collections = this.collectionStore.collections();
        const diffsByMd5 = this.osuDb.diffsByMd5();
        const addedMd5s = this.packStore.addedMd5s();

        return collections
            .map(col => {
                let available = 0;
                let added = 0;
                for (const md5 of col.md5s) {
                    if (diffsByMd5.has(md5)) {
                        available++;
                        if (addedMd5s.has(md5)) added++;
                    }
                }
                return { collection: col, available, added };
            })
            .filter(e => e.available > 0);
    });

    addCollection(entry: CollectionEntry) {
        this.packStore.addFromCollection(entry.collection);
    }
}
