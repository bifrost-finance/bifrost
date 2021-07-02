const fs = require("fs");
const glob = require("glob");
const PATH = require("path");

function readFile(path) {
    try {
        const data = fs.readFileSync(path, "utf-8");

        return data;
    } catch (err) {
        console.error(err);
    }
}

function getFiles(path) {
    if(fs.statSync(path).isDirectory()) {
        return glob.sync(PATH.join(path, "/**/*.json"));
    } else {
        return new Array(path);
    }
}

const arg = process.argv[2];
const files = getFiles(arg);

files.forEach(file => {
    let content = readFile(file);
    let barr = JSON.parse(content).balances;

    if(barr == null) {
        console.error("Invalid Balances Config.");
    }

    let blen = barr.length;

    for(let qptr = 0; qptr < blen - 1; qptr++) {
        for(let sptr = qptr + 1; sptr < blen; sptr++) {
            let qaddr = barr[qptr][0];
            let saddr = barr[sptr][0];

            if(qaddr == saddr) {
                qptr++;

                let tmp = barr[qptr];
                barr[qptr] = barr[sptr];
                barr[sptr] = tmp;

                console.log(`Duplicate: ${qaddr}`);
            }
        }
    }
});