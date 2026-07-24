import { Plugin } from 'vite';

export interface DwarfPluginOptions {
  /** Enable pretty-printed output */
  pretty?: boolean;
  /** Additional compiler options */
  compilerOptions?: Record<string, any>;
}

/**
 * Vite plugin for compiling .kzd (Dwarf) files.
 * 
 * Transforms Dwarf source into JavaScript/TypeScript with HMR and source maps.
 */
export default function dwarfPlugin(options?: DwarfPluginOptions): Plugin;
