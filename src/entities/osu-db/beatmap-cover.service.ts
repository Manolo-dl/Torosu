import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

/** At most this many covers are made at once: each decodes a large image. */
const AT_ONCE = 3;
/** The covers kept (as blob: URLs, a few tens of KB each); the oldest go first. */
const KEPT = 400;

/**
 * The covers of the beatmap sets, made by the backend from each set's own
 * background (`beatmap_cover`), kept on disk there and here as blob: URLs.
 * A card asks for its own as it shows: with a virtual list, only those in
 * sight. `null` for a set with no background.
 */
@Injectable({ providedIn: 'root' })
export class BeatmapCoverService {
    private readonly covers = new Map<string, Promise<string | null>>();
    private readonly waiting: (() => void)[] = [];
    private running = 0;

    /** The cover of the set in `folder`, from the background its `.osu` file `osuFile` names. */
    cover(folder: string, osuFile: string): Promise<string | null> {
        const key = `${folder}\u0000${osuFile}`;
        let cover = this.covers.get(key);
        if (cover) {
            // Used again: kept longer.
            this.covers.delete(key);
        } else {
            cover = this.make(folder, osuFile);
        }
        this.covers.set(key, cover);
        this.forgetOldest();
        return cover;
    }

    private async make(folder: string, osuFile: string): Promise<string | null> {
        await this.turn();
        try {
            const bytes = await invoke<ArrayBuffer>('beatmap_cover', { folder, osuFile });
            return URL.createObjectURL(new Blob([bytes], { type: 'image/jpeg' }));
        } catch {
            // No background, or it can't be read: the card goes without.
            return null;
        } finally {
            this.running--;
            this.waiting.shift()?.();
        }
    }

    /** Waits until fewer than `AT_ONCE` are being made. */
    private turn(): Promise<void> {
        if (this.running < AT_ONCE) {
            this.running++;
            return Promise.resolve();
        }
        return new Promise(resolve =>
            this.waiting.push(() => {
                this.running++;
                resolve();
            }),
        );
    }

    private forgetOldest() {
        while (this.covers.size > KEPT) {
            const [key, oldest] = this.covers.entries().next().value!;
            this.covers.delete(key);
            oldest.then(url => url && URL.revokeObjectURL(url));
        }
    }
}
