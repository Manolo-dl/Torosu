import { Injectable } from '@angular/core';
import { beatmapsetIdFromUrl } from '@shared/utils';

export interface ParsedImport {
    name: string;
    md5s: string[];
    beatmapsetIds: number[];
}

const MD5_RE = /^[a-f0-9]{32}$/i;

@Injectable({
  providedIn: 'root',
})
export class ImportCollectionParserService {

  parse(content: string): ParsedImport[] {
    const collections: ParsedImport[] = [];
    let current: ParsedImport | null = null;

    for (const rawLine of content.split('\n')) {
      const line = rawLine.trim();
      if (!line) continue;

      if (line.startsWith('-')) {
        const name = line.slice(1).trim();
        current = { name, md5s: [], beatmapsetIds: [] };
        collections.push(current);
      } else if (current) {
        const beatmapsetId = beatmapsetIdFromUrl(line);
        if (beatmapsetId !== null) {
          current.beatmapsetIds.push(beatmapsetId);
        } else if (MD5_RE.test(line)) {
          current.md5s.push(line);
        }
      }
    }

    return collections;
  }
}
