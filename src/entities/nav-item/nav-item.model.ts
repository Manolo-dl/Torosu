export interface NavItem {
    /** A translation key. */
    label: string;
    route: string;
    /** Taiga UI icon, for `<tui-icon [icon]>`. */
    icon: string;
    /** What the tool does, for its card on the home page. */
    /** A translation key. */
    description?: string;
    disabled?: boolean;
}