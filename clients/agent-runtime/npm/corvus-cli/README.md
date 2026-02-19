# @dallay/corvus

Node.js launcher for the native Corvus Rust binary.

## Usage

```bash
npx @dallay/corvus --help
pnpm dlx @dallay/corvus status
yarn dlx @dallay/corvus agent -m "Hola"
bunx @dallay/corvus doctor
```

## Install globally

```bash
npm i -g @dallay/corvus
pnpm add -g @dallay/corvus
yarn global add @dallay/corvus
bun add -g @dallay/corvus
```

Then run:

```bash
corvus --help
```

## Binary source

The launcher downloads platform binaries from:

`https://github.com/dallay/corvus/releases/download/v<version>/corvus-<platform>-<arch>`

Override with `CORVUS_NPM_RELEASE_BASE` if you host custom artifacts.
