// Only allow raw imports for safe content extensions — do not import secrets via ?raw
declare module "*.md?raw" {
  const content: string;
  export default content;
}

declare module "*.txt?raw" {
  const content: string;
  export default content;
}
