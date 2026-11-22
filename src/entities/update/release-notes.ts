/** A piece of a line of the notes: plain, bold or code. */
export interface NotePart {
    text: string;
    strong?: boolean;
    code?: boolean;
}

/** A line of the notes, as the update panel shows it. */
export interface NoteBlock {
    kind: 'heading' | 'item' | 'text';
    parts: NotePart[];
}

/** Sections of the release's body that are for the website, not for the app. */
const WEBSITE_SECTIONS = /^downloads?$/i;
/** The release workflow's line under them. */
const WEBSITE_LINES = /updates itself from the app/i;

/**
 * The release notes (the GitHub release's body, in Markdown) for the update
 * panel: headings, list items and lines, with bold and code, links as their
 * text. The downloads the release workflow lists are left out: Torosu
 * updates itself.
 */
export function releaseNotes(notes: string | null): NoteBlock[] {
    const blocks: NoteBlock[] = [];
    let skipping = false;
    for (const raw of (notes ?? '').split(/\r?\n/)) {
        const line = raw.trim();
        if (!line) continue;
        const heading = line.match(/^#{1,6}\s+(.*)$/);
        if (heading) {
            skipping = WEBSITE_SECTIONS.test(heading[1].trim());
            if (!skipping) blocks.push({ kind: 'heading', parts: inline(heading[1]) });
            continue;
        }
        if (skipping || WEBSITE_LINES.test(line)) continue;
        const item = line.match(/^[-*+]\s+(.*)$/);
        blocks.push(item ? { kind: 'item', parts: inline(item[1]) } : { kind: 'text', parts: inline(line) });
    }
    return blocks;
}

function inline(text: string): NotePart[] {
    const plain = text.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1');
    const parts: NotePart[] = [];
    const marks = /\*\*([^*]+)\*\*|`([^`]+)`/g;
    let last = 0;
    for (const match of plain.matchAll(marks)) {
        if (match.index > last) parts.push({ text: plain.slice(last, match.index) });
        parts.push(match[1] !== undefined ? { text: match[1], strong: true } : { text: match[2], code: true });
        last = match.index + match[0].length;
    }
    if (last < plain.length) parts.push({ text: plain.slice(last) });
    return parts;
}
