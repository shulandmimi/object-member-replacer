import { transform } from './binding';
import type { LoaderDefinitionFunction } from 'webpack';

const loader: LoaderDefinitionFunction = function (content) {
    const callback = this.async();

    const result = transform(content, {});

    console.log({ result });
    callback(null, result);
};

module.exports = loader;
