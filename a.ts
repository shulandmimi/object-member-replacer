import zlib, { gzipSync } from "node:zlib";
import fs from "node:fs";

// const gzip = zlib.createGzip({});

const content = fs.readFileSync('./a.ts').toString('utf-8')

const result = gzipSync(content, {});


const uncontent = zlib.gunzipSync(result)

console.log(uncontent.toString('utf-8'))
fs.writeFileSync("hello.gz", result);
