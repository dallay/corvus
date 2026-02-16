# @corvus/cli

Node.js launcher for the native Corvus Rust binary.

## Usage

```bash
npx @corvus/cli --help
pnpm dlx @corvus/cli status
yarn dlx @corvus/cli agent -m "Hola"
bunx @corvus/cli doctor
```

## Install globally

```bash
npm i -g @corvus/cli
pnpm add -g @corvus/cli
yarn global add @corvus/cli
bun add -g @corvus/cli
```

Then run:

```bash
corvus --help
```

## Binary source

The launcher downloads platform binaries from:

`https://github.com/dallay/corvus/releases/download/v<version>/corvus-<platform>-<arch>`

Override with `CORVUS_NPM_RELEASE_BASE` if you host custom artifacts.
