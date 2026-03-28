/** @type {import("stylelint").Config} */
export default {
  extends: ["stylelint-config-standard"],
  ignoreFiles: ["**/coverage/**", "**/dist/**", "**/.astro/**", "**/node_modules/**"],
};
