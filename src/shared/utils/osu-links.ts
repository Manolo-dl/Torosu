/** The page of a beatmap set on the osu! website. */
export function beatmapsetUrl(id: number): string {
  return `https://osu.ppy.sh/beatmapsets/${id}`;
}

/**
 * The id in a beatmap set link (`…/beatmapsets/123`, on the osu! website or
 * a mirror, with or without `https://`), or `null` if it is not one.
 */
export function beatmapsetIdFromUrl(text: string): number | null {
  const match = /beatmapsets\/(\d+)/.exec(text);
  return match ? Number(match[1]) : null;
}
