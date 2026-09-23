/// <reference types="vite/client" />

/**
 * Vite's ambient types, which is what gives an asset import a type.
 *
 * `import mark from "./assets/....svg"` resolves to the URL Vite emits for that
 * file — hashed in a build, served directly in development. Without this
 * reference TypeScript sees an import of a non-module and fails, so the file
 * exists for the compiler rather than for the bundler, which never needed it.
 */
