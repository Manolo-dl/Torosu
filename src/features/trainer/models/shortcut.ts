/** The keys that only change others: alone they are no shortcut. */
const MODIFIER_CODES = new Set([
    'ShiftLeft', 'ShiftRight', 'ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'MetaLeft', 'MetaRight', 'OSLeft', 'OSRight',
]);

/**
 * The shortcut of a key press, as the global-shortcut plugin reads it
 * (`Ctrl+Shift+KeyK`), or `null` for a modifier alone. The key is the one at
 * that place on the keyboard, whatever its layout.
 */
export function shortcutOf(event: KeyboardEvent): string | null {
    if (MODIFIER_CODES.has(event.code) || !event.code) return null;
    const modifiers = [
        event.ctrlKey && 'Ctrl',
        event.altKey && 'Alt',
        event.shiftKey && 'Shift',
        event.metaKey && 'Super',
    ].filter(Boolean);
    return [...modifiers, event.code].join('+');
}

/** Whether `shortcut` has a modifier, or is a function key: anything else is a key of the game. */
export function isSafeShortcut(shortcut: string): boolean {
    return shortcut.includes('+') || /^F\d+$/.test(shortcut);
}

/** How a shortcut is shown: `Ctrl + Shift + K`. */
export function shortcutLabel(shortcut: string): string {
    return shortcut
        .split('+')
        .map(part => part.replace(/^Key(?=[A-Z]$)/, '').replace(/^Digit(?=\d$)/, ''))
        .join(' + ');
}
