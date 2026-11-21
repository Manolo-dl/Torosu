/**
 * osu!'s game modes, by their number in osu!.db and the .osu files: 0 osu!
 * (standard), 1 taiko, 2 catch, 3 mania. `ruleset` is the name the osu!
 * memory reader gives them (`Ruleset`). `key` is the translation of
 * `shortName`, which is English.
 */
export const GAME_MODES = [
  { name: 'osu!', shortName: 'Standard', key: 'modes.standard', ruleset: 'osu', icon: 'assets/icons/modes/osu.svg', color: '#ff66ab' },
  { name: 'osu!taiko', shortName: 'Taiko', key: 'modes.taiko', ruleset: 'taiko', icon: 'assets/icons/modes/taiko.svg', color: '#e05555' },
  { name: 'osu!catch', shortName: 'Catch', key: 'modes.catch', ruleset: 'fruits', icon: 'assets/icons/modes/catch.svg', color: '#66bbff' },
  { name: 'osu!mania', shortName: 'Mania', key: 'modes.mania', ruleset: 'mania', icon: 'assets/icons/modes/mania.svg', color: '#c966ff' },
] as const;

/** The icon of a game mode, for `<app-icon [name]>`; drawn in the text colour. */
export function modeIcon(mode: number): string {
  return (GAME_MODES[mode] ?? GAME_MODES[0]).icon;
}

export function modeColor(mode: number): string {
  return GAME_MODES[mode]?.color ?? '#aaa';
}
