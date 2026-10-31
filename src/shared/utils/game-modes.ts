/**
 * osu!'s game modes, by their number in osu!.db and the .osu files: 0 osu!
 * (standard), 1 taiko, 2 catch, 3 mania. `ruleset` is the name the osu!
 * memory reader gives them (`Ruleset`).
 */
export const GAME_MODES = [
  { name: 'osu!', shortName: 'Standard', ruleset: 'osu', icon: 'assets/icons/modes/osu.svg', color: '#ff66ab' },
  { name: 'osu!taiko', shortName: 'Taiko', ruleset: 'taiko', icon: 'assets/icons/modes/taiko.svg', color: '#e05555' },
  { name: 'osu!catch', shortName: 'Catch', ruleset: 'fruits', icon: 'assets/icons/modes/catch.svg', color: '#66bbff' },
  { name: 'osu!mania', shortName: 'Mania', ruleset: 'mania', icon: 'assets/icons/modes/mania.svg', color: '#c966ff' },
] as const;

/** The icon of a game mode, for `<tui-icon [icon]>`; drawn in the text colour. */
export function modeIcon(mode: number): string {
  return (GAME_MODES[mode] ?? GAME_MODES[0]).icon;
}

export function modeColor(mode: number): string {
  return GAME_MODES[mode]?.color ?? '#aaa';
}
