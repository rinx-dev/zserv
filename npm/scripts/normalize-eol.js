#!/usr/bin/env node
"use strict";

// Runs before the package is packed (npm pack / npm publish): converts CRLF line endings to
// LF in the JavaScript files that ship. A Windows checkout can turn the launcher's first line
// into "#!/usr/bin/env node\r", and then `zserv` fails on Linux and macOS for package managers
// that do not repair it, such as Yarn 1 ("env: 'node\r': No such file or directory").
//
// Usage: node scripts/normalize-eol.js [package directory]

const fs = require("node:fs");
const path = require("node:path");

const root = process.argv[2] ?? path.join(__dirname, "..");

for (const dir of ["bin", "lib"]) {
  for (const name of fs.readdirSync(path.join(root, dir))) {
    if (!name.endsWith(".js")) continue;
    const file = path.join(root, dir, name);
    const text = fs.readFileSync(file, "utf8");
    if (text.includes("\r\n")) {
      fs.writeFileSync(file, text.replace(/\r\n/g, "\n"));
      console.log(`normalize-eol: converted ${dir}/${name} to LF line endings`);
    }
  }
}
