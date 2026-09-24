import { Injectable } from '@angular/core';

export interface ParsedImport {
    name: string;
    md5s: string[];
    beatmapsetIds: number[];
}

const MD5_RE = /^[a-f0-9]{32}$/i;
const BEATMAPSET_URL_RE = /^https?:\/\/osu\.ppy\.sh\/beatmapsets\/(\d+)/;

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
        const urlMatch = BEATMAPSET_URL_RE.exec(line);
        if (urlMatch) {
          current.beatmapsetIds.push(Number(urlMatch[1]));
        } else if (MD5_RE.test(line)) {
          current.md5s.push(line);
        }
      }
    }

    return collections;
  }
}
