export interface NavItem {
    /** A translation key. */
    label: string;
    route: string;
    /** An icon of the set, for `<app-icon [name]>`. */
    icon: string;
    /** What the tool does, for its card on the home page. */
    /** A translation key. */
    description?: string;
    disabled?: boolean;
}