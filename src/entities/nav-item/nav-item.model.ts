import { IconDefinition } from '@fortawesome/fontawesome-svg-core';

export interface NavItem {
    label: string;
    route: string;
    icon: IconDefinition;
    disabled?: boolean;
}