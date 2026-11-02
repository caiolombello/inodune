declare module 'virtual:inodune-demo' {
  const corpus: unknown;
  export const catalog: Record<string, string>;
  export default corpus;
}
/** Build-time flag: false in release builds, so demo-only UI is dropped. */
declare const __INODUNE_DEMO__: boolean;
