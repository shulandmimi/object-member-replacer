import { transform } from "../binding";
import type { LoaderDefinitionFunction } from "webpack";

export const loader: LoaderDefinitionFunction = function (content) {
    const callback = this.async();

    const result = transform(content, {});

    callback(null, result);
};

export default loader;