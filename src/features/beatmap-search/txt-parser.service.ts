import { Injectable } from '@angular/core';

@Injectable({
  providedIn: 'root',
})
export class TxtParserService {

  parse(content: string): number[] {

    const ids: number[] = [];

    for (const line of content.split('\n')) {
      const trimmed = line.trim();

      if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//') || trimmed.startsWith('---')) continue;

      const id = this.extractBeatmapSetId(trimmed);
      if (id) ids.push(id);
    }

    return ids;
  }

  private extractBeatmapSetId(url: string): number | null {
    const match = url.match(/beatmapsets\/(\d+)/);
    if (match) return parseInt(match[1]);

    if (/^\d+$/.test(url)) return parseInt(url);
    
    return null;
  }
}
