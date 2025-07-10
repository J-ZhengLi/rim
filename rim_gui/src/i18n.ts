
import { createI18n, I18n } from 'vue-i18n';
import enUS from '../../locales/en-US.json';
import zhCN from '../../locales/zh-CN.json';

let i18n: I18n | null = null;

// rust-i18n uses Ruby-on-rails styled placeholder `%{}`,
// but vue-i18n uses its own (maybe) placeholder style where the
// `%` is not needed, therefore the percent sign need to be removed
// before passing to vue-i18n.
// NB (J-ZhengLi): I would use custom formatter if the documentation of
// vue-i18n is not that damn limited!
function convertRailsPlaceholders(obj: any): any {
    if (typeof obj === 'string') {
        return obj.replace(/%\{(\w+)\}/g, '{$1}');
    } else if (Array.isArray(obj)) {
        return obj.map(convertRailsPlaceholders);
    } else if (typeof obj === 'object' && obj !== null) {
        const result: Record<string, any> = {};
        for (const key in obj) {
            result[key] = convertRailsPlaceholders(obj[key]);
        }
        return result;
    }
    return obj;
}

export function getOrInitI18n(locale?: string): I18n {
    if (i18n) {
        return i18n;
    }

    i18n = createI18n({
        legacy: false,
        locale,
        messages: {
            'en-US': convertRailsPlaceholders(enUS),
            'zh-CN': convertRailsPlaceholders(zhCN),
        }
    });

    return i18n;
}
