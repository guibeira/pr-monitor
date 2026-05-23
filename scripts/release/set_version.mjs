#!/usr/bin/env node
import fs from "node:fs";

const version = process.argv[2];

if (!version) {
  console.error("usage: set_version.mjs <version>");
  process.exit(1);
}

if (!/^[0-9]+\.[0-9]+\.[0-9]+(-(rc|alpha|beta|pre)[0-9]*)?$/.test(version)) {
  console.error(`invalid version: ${version}`);
  process.exit(1);
}

function writeJson(path, update) {
  const data = JSON.parse(fs.readFileSync(path, "utf8"));
  update(data);
  fs.writeFileSync(path, `${JSON.stringify(data, null, 2)}\n`);
}

writeJson("package.json", (data) => {
  data.version = version;
});

writeJson("package-lock.json", (data) => {
  data.version = version;
  if (data.packages?.[""]) {
    data.packages[""].version = version;
  }
});

writeJson("src-tauri/tauri.conf.json", (data) => {
  data.version = version;
});

const cargoPath = "src-tauri/Cargo.toml";
const cargoToml = fs.readFileSync(cargoPath, "utf8");
const nextCargoToml = cargoToml.replace(
  /(\[package\][\s\S]*?\nversion\s*=\s*)"[^"]*"/,
  `$1"${version}"`,
);

if (nextCargoToml === cargoToml && !cargoToml.includes(`version = "${version}"`)) {
  console.error("failed to locate [package] version in src-tauri/Cargo.toml");
  process.exit(1);
}

fs.writeFileSync(cargoPath, nextCargoToml);
