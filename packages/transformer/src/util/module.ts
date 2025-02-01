import { Filter, ModuleType, OOMPluginOptions } from "../type";

const tsRe = /\.tsx?$/;
const jsRe = /\.jsx?$/;

export function moduleTypeFromName(name: string): ModuleType | undefined {
    if (tsRe.test(name)) {
        return ModuleType.TypeScript;
    }

    if (jsRe.test(name)) {
        return ModuleType.JavaScript;
    }

    return undefined;
}

type FilterOption = Pick<OOMPluginOptions, "include" | "exclude">;

export function filterFactory(filter: Filter): (filename: string) => boolean {
    if (typeof filter === "string") {
        return (filename) => filename.includes(filter);
    }

    if (filter instanceof RegExp) {
        return (filename) => filter.test(filename);
    }

    return filter;
}

export function createFilter(
    options: FilterOption
): (filename: string) => boolean {
    const { include, exclude } = options;

    const includeFilters = include ? include.map(filterFactory) : undefined;
    const excludeFilters = exclude ? exclude.map(filterFactory) : undefined;

    const includeFn = (filename: string) =>
        includeFilters?.some((fn) => fn(filename)) ?? true;
    const excludeFn = (filename: string) =>
        excludeFilters?.some((fn) => fn(filename)) ?? false;

    return (filename: string) => {
        if (!includeFn(filename)) return false;

        if (excludeFn(filename)) return false;

        return true;
    };
}
