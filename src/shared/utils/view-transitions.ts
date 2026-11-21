/**
 * Whether pages and the theme change in View Transitions. Not in WebKitGTK
 * (Tauri on Linux): it has them, but after a couple of minutes it stops
 * freeing the pictures each one takes, some 4 MB a page change on 2.52,
 * gigabytes after a while of going from tool to tool. There a page comes
 * in with a CSS animation instead (styles.css, `data-view-transitions`).
 */
export const VIEW_TRANSITIONS =
    typeof document !== 'undefined' && 'startViewTransition' in document && !/Linux/.test(navigator.userAgent);
