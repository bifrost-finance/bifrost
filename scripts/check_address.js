const fs = require("fs");
const glob = require("glob");
const PATH = require("path");
// Import Polkadot.js API dependencies.
const { decodeAddress, encodeAddress } = require("@polkadot/keyring");
const { hexToU8a, isHex } = require("@polkadot/util");

// Specify an address to test.
const address = "14wCTjDZsrvL6SsEsdErvnvzjfnerbTMefvnTzjBgAzwcK3k";

function checkSS58(address) {
    try {
        encodeAddress(isHex(address) ? hexToU8a(address) : decodeAddress(address));

        return true;
    } catch (err) {
        return false;
    }
}

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

const patt = /(?<=\[")[a-zA-Z0-9]*(?=",)/g;

const arg = process.argv[2];
const files = getFiles(arg);

files.forEach(file => {
    let content = readFile(file);

    let addrs = content.match(patt);

    addrs.forEach(addr => {
        if(!checkSS58(addr)) {
            console.log(`Invalid Addr: ${addr} in ${file}.`);
        }
    });
});