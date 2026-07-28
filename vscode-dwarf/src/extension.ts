import * as path from 'path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration('dwarf.lsp');
  let serverPath = config.get<string>('path') || 'dwarf-lsp';

  if (!path.isAbsolute(serverPath)) {
    serverPath = context.asAbsolutePath(path.join('server', serverPath));
  }

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: ['--stdio'],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'dwarf' }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/*.kzd'),
    },
  };

  client = new LanguageClient(
    'dwarf-lsp',
    'Dwarf Language Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) return undefined;
  return client.stop();
}
