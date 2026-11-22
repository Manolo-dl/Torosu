import { Injectable } from '@angular/core';
import { beatmapsetIdFromUrl } from '@shared/utils';

export interface ParsedImport {
    name: string;
    /** Lowercase, as osu! keeps them, each once. */
    md5s: string[];
    beatmapsetIds: number[];
}

const MD5_RE = /^[a-f0-9]{32}$/i;

/**
 * The text of an import: a collection per block, its name on a line that
 * starts with `-`, then a map per line, as its MD5 or a link to its set
 * (what the exporter writes). Blocks of the same name are one collection.
 */
@Injectable({
  providedIn: 'root',
})
export class ImportCollectionParserService {

  parse(content: string): ParsedImport[] {
    const byName = new Map<string, { md5s: Set<string>; beatmapsetIds: Set<number> }>();
    let current: { md5s: Set<string>; beatmapsetIds: Set<number> } | null = null;

    for (const rawLine of content.split('\n')) {
      const line = rawLine.trim();
      if (!line) continue;

      if (line.startsWith('-')) {
        const name = line.slice(1).trim();
        current = byName.get(name) ?? { md5s: new Set(), beatmapsetIds: new Set() };
        byName.set(name, current);
      } else if (current) {
        const beatmapsetId = beatmapsetIdFromUrl(line);
        if (beatmapsetId !== null) {
          current.beatmapsetIds.add(beatmapsetId);
        } else if (MD5_RE.test(line)) {
          current.md5s.add(line.toLowerCase());
        }
      }
    }

    return [...byName].map(([name, { md5s, beatmapsetIds }]) => ({
      name,
      md5s: [...md5s],
      beatmapsetIds: [...beatmapsetIds],
    }));
  }
}
