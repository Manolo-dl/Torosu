export interface NavItem {
    label: string;
    route: string;
    /** Taiga UI icon, for `<tui-icon [icon]>`. */
    icon: string;
    disabled?: boolean;
}