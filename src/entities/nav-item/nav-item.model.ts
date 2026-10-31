export interface NavItem {
    label: string;
    route: string;
    /** Taiga UI icon, for `<tui-icon [icon]>`. */
    icon: string;
    /** What the tool does, for its card on the home page. */
    description?: string;
    disabled?: boolean;
}