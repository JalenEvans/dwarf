export interface CompileResult {
  success: boolean;
  output: string;
  diagnostics: Diagnostic[];
  outputExtension: string;
}

export interface Diagnostic {
  code: string;
  severity: string;
  message: string;
  file: string | null;
  line: number | null;
  col: number | null;
}

export interface CompileOptions {
  target?: string;
  pretty?: boolean;
  skip_passes?: string[];
}

export function compile(
  source: string,
  filename: string,
  options?: CompileOptions | string
): CompileResult;

export function compileSimple(
  source: string,
  filename: string
): CompileResult;

export function version(): string;
