import { Component, computed, inject, ChangeDetectionStrategy } from '@angular/core';
import { CollectionStore } from '@entities/collection';
import { OsuDbStore } from '@entities/osu-db';
import { PackStore } from '@entities/pack';
import { OsuCollection } from '@entities/collection';

interface CollectionEntry {
    collection: OsuCollection;
    available: number;
    added: number;
}

@Component({
    selector: 'app-pack-collection-adder',
    imports: [],
    templateUrl: './pack-collection-adder.component.html',
    styleUrl: './pack-collection-adder.component.css',
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
