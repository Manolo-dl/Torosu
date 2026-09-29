/** Background colour osu! uses for a star rating. */
export function starBg(stars: number): string {
  if (stars < 2)   return '#8bc9ff';
  if (stars < 2.5) return '#89e066';
  if (stars < 3.5) return '#f7eb40';
  if (stars < 4.5) return '#f9a12e';
  if (stars < 5.5) return '#f9504b';
  if (stars < 6.5) return '#e06fda';
  if (stars < 8)   return '#be4bf5';
  return '#6c30f5';
}

/** Readable text colour on top of {@link starBg}. */
export function starFg(stars: number): string {
  return stars >= 5 ? '#fff' : '#1a1a1a';
}

export function modeChar(mode: number): string {
  return ['●', '◉', '✿', '⊞'][mode] ?? '●';
}

export function modeColor(mode: number): string {
  return ['#ff66ab', '#e05555', '#66bbff', '#c966ff'][mode] ?? '#aaa';
}
