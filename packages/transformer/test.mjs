import loader from './dist/webpack.js';
import assert from 'node:assert';


assert.strictEqual(typeof loader, 'function');
assert.strictEqual(loader.default, loader);