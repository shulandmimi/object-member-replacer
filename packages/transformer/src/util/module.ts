import { ModuleType } from "../type";

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
